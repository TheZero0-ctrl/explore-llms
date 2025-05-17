use quick_xml::{events::{BytesEnd, BytesStart, BytesText, Event}, Reader};
use regex::Regex;
use rig::{completion::{Prompt, ToolDefinition}, providers, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone)]
struct AppState {
    ollama_client: providers::ollama::Client,
}


#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct Paper {
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub url: String,
    pub categories: Vec<String>,
}

impl Paper {
    fn new() -> Self {
        Self {
            title: String::new(),
            authors: Vec::new(),
            abstract_text: String::new(),
            url: String::new(),
            categories: Vec::new(),
        }
    }
}

#[derive(serde::Deserialize)]
pub struct SearchArgs {
    query: String,
    max_results: Option<i32>,
}

// Tool to search for papers
#[derive(serde::Deserialize, serde::Serialize)]
pub struct ArxivSearchTool;

#[derive(Debug, thiserror::Error)]
pub enum ArxivError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("XML parsing error: {0}")]
    XmlParsing(#[from] quick_xml::Error),
    #[error("No results found")]
    NoResults,
    #[error("UTF-8 decoding error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
}

const ARXIV_URL: &str = "http://export.arxiv.org/api/query";

impl Tool for ArxivSearchTool {
    const NAME: &'static str = "search_arxiv";
    type Error = ArxivError;
    type Args = SearchArgs;
    type Output = Vec<Paper>;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "search_arxiv".to_string(),
            description: "Search for academic papers on arXiv".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query for papers"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of results to return (default: 5)"
                    }
                },
                "required": ["query"]
            }),
        }
    }

     async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let max_results = args.max_results.unwrap_or(5);
        let client = reqwest::Client::new();

        let response = client
            .get(ARXIV_URL)
            .query(&[
                ("search_query", format!("all:{}", args.query)),
                ("start", 0.to_string()),
                ("max_results", max_results.to_string()),
            ])
            .send()
            .await?
            .text()
            .await?;

        Ok(ArxivParser::new().parse_response(&response)?)
    }
}

#[derive(Default)]
struct ArxivParser<'a> {
    papers: Vec<Paper>,
    current_paper: Option<Paper>,
    current_authors: Vec<String>,
    current_categories: Vec<String>,
    in_entry: bool,
    current_field: Option<&'a str>,
}

impl<'a> ArxivParser<'a> {
    fn new() -> Self {
        Self {
            papers: Vec::new(),
            current_paper: None,
            current_authors: Vec::new(),
            current_categories: Vec::new(),
            in_entry: false,
            current_field: None,
        }
    }

    fn parse_start_event(&mut self, event: &BytesStart) {
        match event.name().as_ref() {
            // if the tag is "entry", this means we're at the start of a new xml block
            // so we can clear related variables and start anew
            b"entry" => {
                self.in_entry = true;
                self.current_paper = Some(Paper::new());
                self.current_authors.clear();
                self.current_categories.clear();
            }
            // otherwise, change the parsing state
            b"title" if self.in_entry => self.current_field = Some("title"),
            b"author" if self.in_entry => self.current_field = Some("author"),
            b"summary" if self.in_entry => self.current_field = Some("abstract"),
            b"link" if self.in_entry => self.current_field = Some("link"),
            b"category" if self.in_entry => self.current_field = Some("category"),
            _ => (),
        };
    }

    fn parse_text_event(&mut self, event: &BytesText) -> Result<(), ArxivError> {
        // if there's no current paper, just don't return anything
        let Some(paper) = self.current_paper.as_mut() else {
            return Ok(());
        };
        // otherwise, attempt to get the text and fill in the relevant field
        let text = std::str::from_utf8(event.as_ref())?.to_owned();
        match self.current_field {
            Some("title") => paper.title = text,
            Some("author") => self.current_authors.push(text),
            Some("abstract") => paper.abstract_text = text,
            _ => (),
        }
        Ok(())
    }

    fn parse_empty_event(&mut self, event: &BytesStart) -> Result<(), ArxivError> {
        // if we're not in an entry, just don't do anything
        if !self.in_entry {
            return Ok(());
        }
        // if the element is a link, convert the URL to the relevant format
        // and add the URL to the paper
        if event.name().as_ref() == b"link" {
            if let Some(paper) = self.current_paper.as_mut() {
                for attr in event.attributes().flatten() {
                    if attr.key.as_ref() == b"href" {
                        let url = std::str::from_utf8(&attr.value)?;
                        // Convert to HTTPS and ensure PDF URL
                        let secure_url = convert_pdf_url(url);
                        secure_url.clone_into(&mut paper.url);
                    }
                }
            }
        }
        // if the element is a Category, push the category terms
        // into the parser's list of current categories
        if event.name().as_ref() == b"category" {
            for attr in event.attributes().flatten() {
                if attr.key.as_ref() == b"term" {
                    self.current_categories
                        .push(std::str::from_utf8(&attr.value)?.to_owned());
                }
            }
        }

        Ok(())
    }

    fn parse_end_event(&mut self, event: &BytesEnd) -> Result<(), ArxivError> {
        // this is an end event - if the end tag is for an entry
        // add the current paper to the list of papers
        match event.name().as_ref() {
            b"entry" => {
                if let Some(mut paper) = self.current_paper.take() {
                    paper.authors.clone_from(&self.current_authors);
                    paper.categories.clone_from(&self.current_categories);
                    self.papers.push(paper);
                }
                self.in_entry = false;
            }
            // else, just change the currently parsed field to None
            // as there is now nothing to parse
            b"title" | b"author" | b"summary" | b"link" | b"category" => {
                self.current_field = None;
            }
            _ => (),
        }
        Ok(())
    }

    fn parse_response(&mut self, input: &str) -> Result<Vec<Paper>, ArxivError> {
        let mut reader = Reader::from_str(input);
        reader.trim_text(true);

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => self.parse_start_event(e),
                Ok(Event::Text(ref e)) => self.parse_text_event(e)?,
                Ok(Event::Empty(ref e)) => self.parse_empty_event(e)?,
                Ok(Event::End(ref e)) => self.parse_end_event(e)?,
                // EoF means end of file - we can stop trying to parse here
                Ok(Event::Eof) => break,
                Err(e) => return Err(ArxivError::XmlParsing(e)),
                _ => (),
            }
        }

        if self.papers.is_empty() {
            return Err(ArxivError::NoResults);
        }

        Ok(self.papers.clone())
    }
}

fn convert_pdf_url(url: &str) -> String {
    if url.contains("arxiv.org/abs/") {
        // Convert abstract URL to PDF URL
        url.replace("arxiv.org/abs/", "arxiv.org/pdf/")
            .replace("http://", "https://")
            + ".pdf"
    } else if url.contains("arxiv.org/pdf/") {
        // Ensure PDF URL uses HTTPS
        url.replace("http://", "https://")
    } else {
        // Fallback for other URLs
        url.replace("http://", "https://")
    }
}

pub async fn search_papers(model: &str, query: &str) -> Result<Vec<Paper>, anyhow::Error> {
    let client = providers::ollama::Client::new();
    let paper_agent = client
        .agent(model)
        .preamble(r#"
             /no_think
             You are a helpful research assistant that can search and analyze academic papers from arXiv. \
             When asked about a research topic, use the search_arxiv tool to find relevant papers and \
             return only the raw JSON response from the tool.

             **Output Format:**
             [
                 {
                     "title": "Paper Title",
                     "authors": ["Author 1", "Author 2"],
                     "abstract_text": "Abstract Text",
                     "url": "https://arxiv.org/pdf/1234.5678",
                     "categories": ["Category 1", "Category 2"]
                 }
             ]
        "#)
        .tool(ArxivSearchTool)
        .build();

    let response = paper_agent
        .prompt(query)
        .await?;

    let re = Regex::new(r"(?s)<think>.*?</think>\n*").unwrap();
    let result = re.replace_all(&response, "");

    let papers: Vec<Paper> = serde_json::from_str(&result)?;
    Ok(papers)
}

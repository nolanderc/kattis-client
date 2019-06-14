use std::io::{stdin, stdout, BufRead, Write};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Query {
    query: String,
    default_response: Response,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Response {
    Yes,
    No,
}

impl Query {
    pub fn new<S: Into<String>>(query: S) -> Self {
        Query {
            query: query.into(),
            default_response: Response::No,
        }
    }

    pub fn default(&mut self, default: Response) -> &mut Self {
        self.default_response = default;
        self
    }

    pub fn confirm(&self) -> Response {
        print!("{}", self.query);

        let options = match self.default_response {
            Response::Yes => "Y/n",
            Response::No => "y/N",
        };

        print!(" ({}) ", options);
        let _ = stdout().lock().flush();

        let response = stdin()
            .lock()
            .lines()
            .next()
            .and_then(|line| line.ok())
            .and_then(|input| input.parse().ok())
            .unwrap_or(self.default_response.clone());

        response
    }
}

impl FromStr for Response {
    type Err = ();

    fn from_str(text: &str) -> Result<Response, ()> {
        match text {
            "y" | "Y" => Ok(Response::Yes),
            "n" | "N" => Ok(Response::No),
            _ => Err(()),
        }
    }
}

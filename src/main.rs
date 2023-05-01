extern crate core;

use axum::extract::FromRef;
use axum::response::{IntoResponse, Response};
use axum::{
    routing::{get, post},
    Form, Router,
};
use axum_template::{engine::Engine, Key, RenderHtml};
use either::{Either, Left, Right};
use minijinja::{Environment, Source};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
struct InitialPageState {
    prompt: String,
    n: i8,
    size: Size,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(try_from = "String")]
enum Size {
    S256x256,
    S512x512,
    S1024x1024,
}

impl Serialize for Size {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            Size::S256x256 => "256x256",
            Size::S512x512 => "512x512",
            Size::S1024x1024 => "1024x1024",
        })
    }
}

impl TryFrom<String> for Size {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value == "256" {
            return Ok(Size::S256x256);
        }
        if value == "512" {
            return Ok(Size::S512x512);
        }
        if value == "1024" {
            return Ok(Size::S1024x1024);
        }
        Err(value)
    }
}

#[derive(Debug, Serialize)]
struct OpenAiBodyInput {
    prompt: String,
    n: i8,
    size: Size,
}

impl From<&InitialPageState> for OpenAiBodyInput {
    fn from(state: &InitialPageState) -> Self {
        OpenAiBodyInput {
            prompt: state.prompt.to_owned(),
            n: state.n,
            size: state.size.to_owned(),
        }
    }
}

type AppEngine = Engine<Environment<'static>>;

#[derive(Clone, FromRef)]
struct AppState {
    engine: AppEngine,
}

#[tokio::main]
async fn main() {
    let mut jinja = Environment::new();
    jinja.set_source(Source::from_path("templates"));
    let app = Router::new()
        .route("/", get(home))
        .route("/openai", post(call))
        .with_state(AppState {
            engine: Engine::from(jinja),
        });
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn home(engine: AppEngine) -> impl IntoResponse {
    RenderHtml(Key("home.html".to_owned()), engine, ())
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiOutput {
    data: Option<Vec<Line>>,
    error: Option<OpenAiError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Line {
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiError {
    message: String,
}

#[derive(Debug, Serialize)]
struct PageStateWithError {
    prompt: String,
    n: i8,
    size: Size,
    error: String,
}

#[derive(Debug, Serialize)]
struct PageStateWithResults {
    prompt: String,
    n: i8,
    size: Size,
    data: Vec<Line>,
}

struct PageState {
    either: Either<PageStateWithError, PageStateWithResults>,
}

impl From<(InitialPageState, OpenAiOutput)> for PageState {
    fn from(pair: (InitialPageState, OpenAiOutput)) -> Self {
        if pair.1.error.is_some() {
            PageState {
                either: Left(PageStateWithError {
                    prompt: pair.0.prompt,
                    n: pair.0.n,
                    size: pair.0.size,
                    error: pair.1.error.unwrap().message,
                }),
            }
        } else {
            PageState {
                either: Right(PageStateWithResults {
                    prompt: pair.0.prompt,
                    n: pair.0.n,
                    size: pair.0.size,
                    data: pair.1.data.unwrap(),
                }),
            }
        }
    }
}

async fn call(engine: AppEngine, Form(state): Form<InitialPageState>) -> impl IntoResponse {
    let client = reqwest::Client::new();
    let endoint = "https://api.openai.com/v1/images/generations";
    let token = env::var("OPENAI_TOKEN");
    if token.is_err() {
        panic!("No OpenAI token defined. Remember to set the OPENAI_TOKEN environment variable");
    }
    let token = token.unwrap();
    let response = client
        .post(endoint)
        .bearer_auth(token)
        .json(&OpenAiBodyInput::from(&state))
        .send()
        .await
        .unwrap();
    let output = response.json::<OpenAiOutput>().await.unwrap();
    let page_state = PageState::from((state, output));
    match page_state.either {
        Left(data) => Ok(RenderHtml("home.html", engine, data)),
        Right(data) => Err(RenderHtml("home.html", engine, data)
    }
}

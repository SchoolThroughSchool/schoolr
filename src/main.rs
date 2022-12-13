use chrono::naive::NaiveDateTime;
use color_eyre::Result;
use rust_bert::pipelines::question_answering::{Answer, QaInput, QuestionAnsweringModel};

mod classroom;
use classroom::Classroom;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let key = yup_oauth2::read_service_account_key("credentials.json").await?;

    Classroom::list();

    Ok(())
}

fn _due_from_desc(desc: impl Into<String>) -> Result<Option<NaiveDateTime>> {
    let qa_model = QuestionAnsweringModel::new(Default::default())?;

    let question = String::from("When is it due?");

    let Answer { answer, score, .. } = qa_model
        .predict(
            &[QaInput {
                question,
                context: desc.into(),
            }],
            1,
            32,
        )
        .pop()
        .unwrap()
        .pop()
        .unwrap(); // unwrapping is okay here because they give us non-empty vectors

    if score <= 0.5 {
        Ok(None)
    } else {
        Ok(fuzzydate::parse(&answer))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct N;
impl std::fmt::Display for N {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unwrapped a `None`")
    }
}
impl std::error::Error for N {}

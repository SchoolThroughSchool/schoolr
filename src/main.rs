extern crate google_classroom1 as _classroom;

use _classroom::{api::ListCoursesResponse, hyper, hyper_rustls, oauth2};
use chrono::{naive::NaiveDateTime, NaiveDate};
use color_eyre::Result;
use rust_bert::pipelines::question_answering::{Answer, QaInput, QuestionAnsweringModel};
use tokio::task::spawn_blocking;
use tracing::{debug, info};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    color_eyre::install()?;

    let key = oauth2::read_application_secret("credentials.json").await?;
    let auth = oauth2::InstalledFlowAuthenticator::builder(
        key,
        oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk(dirs::config_dir().unwrap().join("tokenscache.json"))
    .build()
    .await?;

    let hub = _classroom::Classroom::new(
        hyper::Client::builder().build(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()
                .https_or_http()
                .enable_http1()
                .enable_http2()
                .build(),
        ),
        auth,
    );

    let c = hub.courses();

    let (
        _,
        ListCoursesResponse {
            courses: Some(courses),
            ..
        },
    ) = c.list().doit().await? else {
        unreachable!()
    };
    debug!("got {} courses", courses.len());

    for course in courses {
        let id = course.id.unwrap();
        let name = course.name.unwrap();
        info!("course: {name} (id: {id})",);

        let work = if let Some(c) = c.course_work_list(&id).doit().await?.1.course_work {
            c
        } else {
            continue;
        };

        for work in work {
            let title = work.title.unwrap();
            let due = if let Some(date) = work.due_date {
                NaiveDate::from_ymd_opt(
                    date.year.unwrap() as _,
                    date.month.unwrap() as _,
                    date.day.unwrap() as _,
                )
            } else if let Some(d) = work.description {
                let title = title.clone();
                spawn_blocking(|| due_from_desc(d, title).ok().flatten().map(|d| d.date())).await?
            } else {
                None
            };

            info!("{}: {:?}", title, due);
        }
    }

    // Classroom::list();

    Ok(())
}

fn due_from_desc(
    desc: impl Into<String>,
    title: impl Into<String>,
) -> Result<Option<NaiveDateTime>> {
    let qa_model = QuestionAnsweringModel::new(Default::default())?;

    let question = String::from("When is it due?");

    let Answer { answer, score, .. } = qa_model
        .predict(
            &[QaInput {
                question,
                context: format!("title: {}\ndescription: {}", title.into(), desc.into()),
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

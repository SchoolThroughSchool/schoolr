extern crate google_classroom1 as _classroom;

use _classroom::{api::ListCoursesResponse, hyper, hyper_rustls, oauth2};
use color_eyre::Result;
use futures::future::join_all;
use tracing::debug;

type Classroom = _classroom::Classroom<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>;

mod course;
use course::Course;

#[tokio::main]
async fn main() -> Result<()> {
    {
        let e = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
        std::env::set_var("RUST_LOG", format!("{e},cached_path=off"));
    }

    tracing_subscriber::fmt::init();
    color_eyre::install()?;

    debug!("getting credentials and authenticating");

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

    debug!("getting courses");

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

    for course in join_all(courses.into_iter().map(|c| Course::new(c, hub.clone())))
        .await
        .into_iter()
        .filter_map(Result::ok)
    {
        serde_json::to_writer_pretty(
            std::fs::File::create(format!("course-{}.json", course.id))?,
            &course,
        )?;
    }

    Ok(())
}

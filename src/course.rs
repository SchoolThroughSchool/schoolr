use crate::Classroom;
use _classroom::api::{Announcement, Course as _Course, CourseWork, Date};
use chrono::NaiveDate;
use color_eyre::Result;
use futures::future::join_all;
use rust_bert::pipelines::question_answering::{QaInput, QuestionAnsweringModel};
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;
use tracing::{debug, trace};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Course {
    pub id: u128,
    pub name: String,
    pub description: String,
    pub teacher: u128,
    pub work: Vec<Work>,
}
impl Course {
    pub async fn new(c: _Course, hub: Classroom) -> Result<Self> {
        let id = c.id.unwrap();
        let name = c.name.unwrap();
        trace!(?name, "getting course work...");
        let mut work = if let Some(w) = hub
            .courses()
            .course_work_list(&id)
            .doit()
            .await?
            .1
            .course_work
        {
            join_all(w.into_iter().map(Work::new)).await
        } else {
            Vec::new()
        };
        trace!(?name, "got {} assignments", work.len());

        trace!(?name, "getting announcements...");
        let work = if let Some(a) = hub
            .courses()
            .announcements_list(&id)
            .doit()
            .await?
            .1
            .announcements
        {
            let announcements = join_all(a.into_iter().map(Work::from_announcement))
                .await
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            work.extend(announcements);
            work
        } else {
            work
        };

        Ok(Self {
            id: id.parse().unwrap(),
            name,
            description: c.description.unwrap_or_default(),
            teacher: c.owner_id.unwrap().parse().unwrap(),
            work,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Work {
    pub link: String,
    pub title: String,
    pub description: String,
    pub due: NaiveDate,
    pub test: bool,
}
impl Work {
    pub async fn new(w: CourseWork) -> Self {
        let due = due(w.description.as_ref(), w.due_date, w.update_time)
            .await
            .unwrap();

        let title = w.title.unwrap();
        let description = w.description.unwrap_or_default();

        Self {
            test: is_test(&description, Some(&title)),
            title,
            description,
            link: w.alternate_link.unwrap(),
            due,
        }
    }

    pub async fn from_announcement(a: Announcement) -> Option<Self> {
        let text = a.text?;

        let due = due(Some(&text), None, None).await?;

        Some(Self {
            test: is_test(&text, None::<&str>),
            link: a.alternate_link.unwrap(),
            title: "Announcement".to_string(),
            description: text,
            due,
        })
    }
}

fn is_test<S: AsRef<str>>(desc: impl AsRef<str>, title: Option<S>) -> bool {
    ["test", "exam", "quiz"].iter().any(|s| {
        desc.as_ref().contains(s)
            || title
                .as_ref()
                .map(|t| t.as_ref().contains(s))
                .unwrap_or(false)
    })
}

async fn due<S: Into<String>>(
    desc: Option<S>,
    date: Option<Date>,
    upd_time: Option<String>,
) -> Option<NaiveDate> {
    let due = if let Some(date) = date {
        NaiveDate::from_ymd_opt(
            date.year.unwrap(),
            date.month.unwrap() as _,
            date.day.unwrap() as _,
        )
    } else if let Some(desc) = desc.map(Into::into) {
        // spawn_blocking(|| due_from_desc(desc))
        //     .await
        //     .ok()
        //     .and_then(|r| r.ok())
        //     .flatten() // TODO: use A.I. to find due dates from descriptions
        None
    } else {
        None
    };

    if let (Some(u), None) = (upd_time, due) {
        NaiveDate::parse_from_str(&u, "%+").ok()
    } else {
        due // if we have a due date, use that
    }
}

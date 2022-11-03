use anyhow::anyhow;
use anyhow::Result;
use html_escape::decode_html_entities;
use once_cell::sync::OnceCell;
use serde::Deserialize;

static PROBLEMS: OnceCell<Vec<Problem>> = OnceCell::new();

#[derive(Debug, Deserialize)]
pub struct ProblemStat {
    #[serde(rename = "question_id")]
    pub id: u32,
    #[serde(rename = "question__title_slug")]
    pub slug: String,
    #[serde(rename = "question__title")]
    pub title: String,
    #[serde(rename = "total_acs")]
    pub accept: u32,
    #[serde(rename = "total_submitted")]
    pub submit: u32,
}

#[derive(Debug, Deserialize)]
pub struct Difficulty {
    pub level: u32,
}

#[derive(Debug, Deserialize)]
pub struct Problem {
    pub stat: ProblemStat,
    pub difficulty: Difficulty,
    pub paid_only: bool,
}

impl Problem {
    pub fn to_message(&self) -> String {
        format!(
            "https://leetcode.com/problems/{} - {}",
            self.stat.slug, self.stat.title
        )
    }

    pub fn slug(&self) -> &str {
        &self.stat.slug
    }

    pub fn url(&self) -> String {
        format!("https://leetcode.com/problems/{}", self.stat.slug)
    }

    pub fn title(&self) -> &str {
        &self.stat.title
    }

    pub fn difficulty(&self) -> &str {
        match self.difficulty.level {
            1 => "Easy",
            2 => "Medium",
            3 => "Hard",
            _ => "Unknown",
        }
    }

    pub fn stats(&self) -> String {
        let rate = self.stat.accept as f64 / self.stat.submit as f64 * 100.00;
        format!("{:.2}%", rate)
    }
}

fn load_problems() -> Vec<Problem> {
    let mut result: serde_json::Value =
        reqwest::blocking::get("https://leetcode.com/api/problems/all/")
            .expect("to load problems from leetcode")
            .json()
            .expect("to be valid JSON");

    serde_json::from_value::<Vec<Problem>>(
        result
            .get_mut("stat_status_pairs")
            .expect("to contain stat_status_pairs")
            .take(),
    )
    .expect("to be valid problems")
    .into_iter()
    .filter(|p| !p.paid_only)
    .collect()
}

pub fn problems() -> &'static Vec<Problem> {
    PROBLEMS.get_or_init(|| load_problems())
}

pub fn get_description(slug: &str) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let result: serde_json::Value = client.post("https://leetcode.com/graphql/")
        .json(&serde_json::json!({
            "query": "    query questionContent($titleSlug: String!) {  question(titleSlug: $titleSlug) {    content    mysqlSchemas  }}    ",
            "variables": {
                "titleSlug": slug
            }
        }))
        .send()?
        .json()?;
    let html = result
        .get("data")
        .and_then(|d| d.get("question"))
        .and_then(|q| q.get("content"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("unable to load problem description"))?;

    let html = format!(r#"<div id="root">{}</div>"#, html);
    let parsed = tl::parse(&html, tl::ParserOptions::default())?;
    let parser = parsed.parser();
    Ok(decode_html_entities(
        &parsed
            .get_element_by_id("root")
            .ok_or_else(|| anyhow!("unable to find root div"))?
            .get(parser)
            .ok_or_else(|| anyhow!("failed to parse html"))?
            .inner_text(parser),
    )
    .lines()
    .filter(|x| !x.trim().is_empty())
    .collect::<Vec<&str>>()
    .join("\n\n")
    .split(|c: char| c == ' ')
    .take(50)
    .collect::<Vec<&str>>()
    .join(" "))
}

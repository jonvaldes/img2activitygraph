use anyhow::{Context, Result};
use clap::{arg, command};
use std::{io::Write, process::Command};

fn git(params: &[&str]) -> Result<()> {
    let output = Command::new("git").args(params).output()?;
    std::io::stdout().write_all(&output.stderr)?;

    Ok(())
}

fn main() -> Result<()> {
    let matches = command!() // requires `cargo` feature
        .arg(arg!(-i --image <FILE> "Image to use").required(true))
        .arg(arg!(-n --username <NAME> "Git user name").required(false))
        .arg(arg!(-m --email <EMAIL> "Git user email").required(false))
        .arg(arg!(-d --density <VALUE> "Density of commits to generate").required(false))
        .get_matches();

    println!("Loading image...");
    let commit_history = image_to_commit_history(matches.get_one::<String>("image").unwrap())?;

    println!("Initializing git repo...");
    let repo_dir = "graphrepo";
    let _ = std::fs::remove_dir_all(repo_dir);
    std::fs::create_dir_all(repo_dir)?;
    std::env::set_current_dir(repo_dir)?;

    git(&["init", "."])?;
    if let Some(username) = matches.get_one::<String>("username") {
        git(&["config", "user.name", username])?;
    }
    if let Some(email) = matches.get_one::<String>("email") {
        git(&["config", "user.email", email])?;
    }

    let mut max_commits = 0;

    let graph_density = if let Some(density) = matches.get_one::<String>("density") {
        density
            .parse::<f32>()
            .context("Couldn't parse density value")?
            * 0.01
    } else {
        0.03
    };

    for x in 0..commit_history.weeks.len() {
        for y in 0..7 {
            let day = &commit_history.weeks[x][y];

            let commit_count = (day.commit_count as f32 * graph_density) as i32;
            println!("Date: {:?} -- {} commits", day.date, commit_count);

            std::env::set_var("GIT_COMMITTER_DATE", &day.date);

            max_commits = max_commits.max(commit_count);
            for _i in 0..commit_count {
                git(&["commit", "--allow-empty", "-m", ".", "--date", &day.date])?;
            }
        }
    }

    println!("Max commits in a single day: {}", max_commits);

    Ok(())
}

#[derive(Clone, Default, Debug)]
struct DayCommits {
    date: String,
    commit_count: u32,
}

type WeekCommits = [DayCommits; 7];

#[derive(Debug)]
struct CommitHistory {
    weeks: Vec<WeekCommits>,
}

impl CommitHistory {
    fn new(weeks: usize) -> Self {
        CommitHistory {
            weeks: vec![Default::default(); weeks],
        }
    }
}

fn image_to_commit_history(path: &str) -> Result<CommitHistory> {
    let img = image::io::Reader::open(path)
        .with_context(|| format!("Could not find file {path}"))?
        .decode()?
        .into_luma8();
    let (w, h) = img.dimensions();
    if h != 7 {
        return Err(anyhow::anyhow!(
            "Image size is incorrect. Only images of height 7 are supported"
        ));
    }

    let mut result = CommitHistory::new(w as usize);

    let today = chrono::offset::Utc::now().date_naive();

    let this_week = today.week(chrono::Weekday::Sun);
    let last_sunday = this_week.first_day();

    let week_count = w;
    let mut current_week_start = last_sunday
        .checked_sub_days(chrono::Days::new(7 * week_count as u64))
        .unwrap();

    for x in 0..w {
        let week_start = current_week_start;

        current_week_start = current_week_start
            .checked_add_days(chrono::Days::new(7))
            .unwrap();

        for y in 0..h {
            let date = week_start
                .checked_add_days(chrono::Days::new(y as u64))
                .unwrap();

            let date_str = date.format("%Y-%m-%d 00:00:00").to_string();

            result.weeks[x as usize][y as usize].commit_count = img.get_pixel(x, y).0[0] as u32;
            result.weeks[x as usize][y as usize].date = date_str;
        }
    }

    Ok(result)
}

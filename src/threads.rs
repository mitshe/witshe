use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreadStatus {
    Active,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,
    pub name: String,
    pub jira_id: Option<String>,
    pub tag: Option<String>,
    pub desc: Option<String>,
    pub status: ThreadStatus,
    pub repo_path: String,
    pub worktree_path: String,
    pub has_worktree: bool,
    pub created_at: String,
}

impl Thread {
    pub fn new(name: String, repo_path: String, worktree_path: String, has_worktree: bool, tag: Option<String>, desc: Option<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            jira_id: None,
            tag,
            desc,
            status: ThreadStatus::Active,
            repo_path,
            worktree_path,
            has_worktree,
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

pub struct Threads {
    threads: Vec<Thread>,
}

impl Threads {
    fn file_path() -> PathBuf {
        let dir = dirs::home_dir().unwrap().join(".witshe");
        fs::create_dir_all(&dir).unwrap();
        dir.join("threads.json")
    }

    pub fn load() -> Self {
        let path = Self::file_path();
        let threads = if path.exists() {
            let content = fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };
        Self { threads }
    }

    pub fn save(&self) {
        let path = Self::file_path();
        let content = serde_json::to_string_pretty(&self.threads).unwrap();
        fs::write(path, content).unwrap();
    }

    pub fn add(&mut self, thread: Thread) {
        self.threads.push(thread);
    }

    pub fn list(&self) -> &[Thread] {
        &self.threads
    }

    pub fn get(&self, name: &str) -> Option<&Thread> {
        self.threads.iter().find(|t| t.name == name)
    }

    pub fn mark_done(&mut self, name: &str) -> bool {
        if let Some(t) = self.threads.iter_mut().find(|t| t.name == name) {
            t.status = ThreadStatus::Done;
            true
        } else {
            false
        }
    }

    pub fn clear_done(&mut self) -> usize {
        let before = self.threads.len();
        self.threads.retain(|t| !matches!(t.status, ThreadStatus::Done));
        before - self.threads.len()
    }

    pub fn rename(&mut self, name: &str, new_name: &str) -> bool {
        if let Some(t) = self.threads.iter_mut().find(|t| t.name == name) {
            t.name = new_name.to_string();
            true
        } else {
            false
        }
    }

    pub fn set_tag(&mut self, name: &str, tag: &str) -> bool {
        if let Some(t) = self.threads.iter_mut().find(|t| t.name == name) {
            t.tag = Some(tag.to_string());
            true
        } else {
            false
        }
    }

    pub fn set_desc(&mut self, name: &str, desc: &str) -> bool {
        if let Some(t) = self.threads.iter_mut().find(|t| t.name == name) {
            t.desc = Some(desc.to_string());
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, name: &str) {
        self.threads.retain(|t| t.name != name);
    }
}

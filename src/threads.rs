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
pub struct Repo {
    pub repo_path: String,
    pub worktree_path: String,
    pub branch: String,
    pub has_worktree: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,
    pub name: String,
    pub jira_id: Option<String>,
    pub tag: Option<String>,
    pub desc: Option<String>,
    pub status: ThreadStatus,
    pub repos: Vec<Repo>,
    pub cwd: Option<String>,
    pub created_at: String,

    // Legacy fields for migration
    #[serde(skip_serializing, default)]
    repo_path: Option<String>,
    #[serde(skip_serializing, default)]
    worktree_path: Option<String>,
    #[serde(skip_serializing, default)]
    has_worktree: Option<bool>,
}

impl Thread {
    pub fn new(name: String, tag: Option<String>, desc: Option<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            jira_id: None,
            tag,
            desc,
            status: ThreadStatus::Active,
            repos: Vec::new(),
            cwd: None,
            created_at: Utc::now().to_rfc3339(),
            repo_path: None,
            worktree_path: None,
            has_worktree: None,
        }
    }

    pub fn add_repo(&mut self, repo: Repo) {
        self.repos.push(repo);
    }

    /// Migrate legacy single-repo format to new repos vec
    fn migrate(&mut self) {
        if self.repos.is_empty() {
            if let (Some(rp), Some(wp)) = (self.repo_path.take(), self.worktree_path.take()) {
                let hw = self.has_worktree.take().unwrap_or(true);
                self.repos.push(Repo {
                    repo_path: rp,
                    worktree_path: wp.clone(),
                    branch: self.name.clone(),
                    has_worktree: hw,
                });
                if self.cwd.is_none() {
                    self.cwd = Some(wp);
                }
            }
        }
    }

    pub fn first_cwd(&self) -> Option<String> {
        self.cwd.clone()
            .or_else(|| self.repos.first().map(|r| r.worktree_path.clone()))
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
        let mut threads: Vec<Thread> = if path.exists() {
            let content = fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };

        // Migrate legacy format
        let mut migrated = false;
        for t in &mut threads {
            if t.repo_path.is_some() || t.worktree_path.is_some() {
                t.migrate();
                migrated = true;
            }
        }

        let store = Self { threads };
        if migrated {
            store.save();
        }
        store
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

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Thread> {
        self.threads.iter_mut().find(|t| t.name == name)
    }

    pub fn mark_done(&mut self, name: &str) -> bool {
        if let Some(t) = self.threads.iter_mut().find(|t| t.name == name) {
            t.status = ThreadStatus::Done;
            true
        } else {
            false
        }
    }

    pub fn reopen(&mut self, name: &str) -> bool {
        if let Some(t) = self.threads.iter_mut().find(|t| t.name == name) {
            t.status = ThreadStatus::Active;
            true
        } else {
            false
        }
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

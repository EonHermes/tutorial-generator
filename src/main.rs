use clap::Parser;
use git2::{Oid, Repository, Sort, Commit, Tree, TreeEntry, Delta};
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use chrono::{DateTime, Utc};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(required = true)]
    repo: String,
    #[arg(short, long)]
    output: Option<PathBuf>,
    #[arg(short, long)]
    title: Option<String>,
    #[arg(short, long)]
    description: Option<String>,
    #[arg(short, long, default_value = "50")]
    max_commits: usize,
    #[arg(short, long, default_value = "markdown")]
    format: String,
    #[arg(short, long)]
    clone: bool,
    #[arg(short, long)]
    dir: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let output_format = match args.format.to_lowercase().as_str() {
        "markdown" | "md" => OutputFormat::Markdown,
        "json" => OutputFormat::Json,
        _ => {
            eprintln!("Invalid format. Use 'markdown' or 'json'.");
            std::process::exit(1);
        }
    };

    let analyzer = if args.clone || args.dir.is_some() {
        let clone_dir = args.dir.unwrap_or_else(|| {
            let name = args.repo.split('/').last().unwrap_or("repo").replace(".git", "");
            std::env::temp_dir().join("tut-gen-").join(name)
        });
        println!("Cloning into {:?}...", &clone_dir);
        let analyzer = GitAnalyzer::clone(&args.repo, Some(&clone_dir))?;
        println!("Clone complete.");
        analyzer
    } else {
        let path = Path::new(&args.repo);
        if !path.exists() {
            eprintln!("Error: Path does not exist: {}", args.repo);
            std::process::exit(1);
        }
        GitAnalyzer::open(path)?
    };

    println!("Analyzing repository...");
    let generator = TutorialGenerator::new(&analyzer);
    let tutorial = generator.generate(args.max_commits)?;

    let output = match output_format {
        OutputFormat::Markdown => tutorial.to_markdown(
            args.title.as_deref(),
            args.description.as_deref()
        ),
        OutputFormat::Json => tutorial.to_json()?,
    };

    if let Some(out_path) = &args.output {
        std::fs::write(out_path, output)?;
        println!("Output written to {}", out_path.display());
    } else {
        println!("\n{}", output);
    }

    Ok(())
}

struct GitAnalyzer {
    repo_path: PathBuf,
    repo: Repository,
}

impl GitAnalyzer {
    fn open<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let repo_path = path.as_ref().to_path_buf();
        let repo = Repository::open(&repo_path)?;
        Ok(Self { repo_path, repo })
    }

    fn clone(url: &str, local_path: Option<&Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let path = match local_path {
            Some(p) => p.to_path_buf(),
            None => {
                let name = url.split('/').last().unwrap_or("repo").replace(".git", "");
                std::env::current_dir()?.join(name)
            }
        };
        let repo = Repository::clone(url, &path)?;
        Ok(Self { repo_path: path, repo })
    }

    fn get_repo_info(&self) -> Result<RepoInfo, Box<dyn std::error::Error>> {
        let total_commits = {
            let mut walk = self.repo.revwalk()?;
            walk.push_head()?;
            walk.count()
        };

        let mut languages = HashSet::new();
        if let Ok(tree) = self.repo.head()?.peel_to_tree() {
            self.walk_tree(&tree, &mut |entry| {
                if entry.filemode() & (git2::FileMode::Blob as i32) != 0 {
                    if let Some(lang) = detect_language(entry.name().unwrap_or("")) {
                        languages.insert(lang.to_string());
                    }
                }
                Ok(())
            })?;
        }

        let mut lang_vec: Vec<String> = languages.into_iter().collect();
        lang_vec.sort();

        let name = self.repo_path.file_name()
            .and_then(|n| n.to_str())
            .ok_or("Invalid repo path")?
            .to_string();

        Ok(RepoInfo {
            name,
            total_commits,
            languages: lang_vec,
        })
    }

    fn get_commit_history(&self, limit: usize) -> Result<Vec<CommitSummary>, Box<dyn std::error::Error>> {
        let mut walk = self.repo.revwalk()?;
        walk.push_head()?;
        walk.set_sorting(Sort::TIME)?;

        let mut commits = Vec::new();
        for (i, oid_res) in walk.enumerate() {
            if i >= limit { break; }
            let oid = oid_res?;
            let commit = self.repo.find_commit(oid)?;

            commits.push(CommitSummary {
                hash: oid.to_string(),
                author: commit.author().name().unwrap_or("Unknown").to_string(),
                message: commit.message().unwrap_or("").trim().to_string(),
            });
        }
        Ok(commits)
    }

    fn get_commit_details(&self, commit_hash: &str) -> Result<CommitDetails, Box<dyn std::error::Error>> {
        let oid = Oid::from_str(commit_hash)?;
        let commit = self.repo.find_commit(oid)?;
        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let mut diff = self.repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
        let mut files = Vec::new();

        diff.foreach(&mut |delta, _| {
            let change_type = ChangeType::from_delta(delta.status());
            let new_path = delta.new_file().path().unwrap_or_else(|| Path::new(""));
            files.push(FileChange {
                path: new_path.to_string_lossy().into_owned(),
                change_type,
                language: detect_language(new_path.to_str().unwrap_or("")).map(String::from),
            });
            true
        }, None, None, None)?;

        Ok(CommitDetails {
            message: commit.message().unwrap_or("").trim().to_string(),
            files,
        })
    }

    fn get_file_content(&self, file_path: &str, commit_hash: &str) -> Result<String, Box<dyn std::error::Error>> {
        let oid = Oid::from_str(commit_hash)?;
        let commit = self.repo.find_commit(oid)?;
        let tree = commit.tree()?;
        let entry = tree.get_path(Path::new(file_path))?;
        let object = self.repo.find_object(entry.id(), None)?;
        let blob = object.as_blob().ok_or_else(|| "Not a blob file")?;
        Ok(String::from_utf8(blob.content().to_vec())?)
    }

    fn walk_tree<F>(&self, tree: &Tree, f: &mut F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(&TreeEntry) -> Result<(), Box<dyn std::error::Error>>,
    {
        for entry in tree.iter() {
            f(&entry)?;
            if entry.filemode() & (git2::FileMode::Tree as i32) != 0 {
                if let Ok(obj) = entry.to_object(&self.repo) {
                    if let Some(subtree) = obj.as_tree() {
                        self.walk_tree(subtree, f)?;
                    }
                }
            }
        }
        Ok(())
    }
}

fn detect_language(path: &str) -> Option<&'static str> {
    use std::path::Path;
    Path::new(path).extension().and_then(|e| e.to_str()).and_then(|ext| match ext {
        "rs" => Some("rust"),
        "js" => Some("javascript"),
        "ts" => Some("typescript"),
        "jsx" | "tsx" => Some("react"),
        "py" => Some("python"),
        "java" => Some("java"),
        "c" => Some("c"),
        "cpp" | "cc" => Some("cpp"),
        "h" | "hpp" => Some("cpp"),
        "go" => Some("go"),
        "rb" => Some("ruby"),
        "php" => Some("php"),
        "html" => Some("html"),
        "css" => Some("css"),
        "scss" => Some("scss"),
        "sql" => Some("sql"),
        "md" => Some("markdown"),
        "json" => Some("json"),
        "yml" | "yaml" => Some("yaml"),
        "toml" => Some("toml"),
        "sh" | "bash" => Some("bash"),
        "lua" => Some("lua"),
        "swift" => Some("swift"),
        "kt" => Some("kotlin"),
        "dart" => Some("dart"),
        "vue" => Some("vue"),
        "svelte" => Some("svelte"),
        "elm" => Some("elm"),
        "hs" => Some("haskell"),
        "clj" => Some("clojure"),
        "ex" => Some("elixir"),
        "fs" => Some("fsharp"),
        _ => None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum OutputFormat {
    Markdown,
    Json,
}

#[derive(Debug, Clone)]
struct RepoInfo {
    name: String,
    total_commits: usize,
    languages: Vec<String>,
}

#[derive(Debug, Clone)]
struct CommitSummary {
    hash: String,
    author: String,
    message: String,
}

#[derive(Debug)]
struct CommitDetails {
    message: String,
    files: Vec<FileChange>,
}

#[derive(Debug, Clone)]
struct FileChange {
    path: String,
    change_type: ChangeType,
    language: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
}

impl ChangeType {
    fn from_delta(delta: Delta) -> Self {
        match delta {
            Delta::Added => ChangeType::Added,
            Delta::Modified => ChangeType::Modified,
            Delta::Deleted => ChangeType::Deleted,
            Delta::Renamed => ChangeType::Renamed,
            Delta::Copied => ChangeType::Added,
            Delta::Ignored => ChangeType::Modified,
            Delta::Untracked => ChangeType::Added,
            Delta::Typechange => ChangeType::Modified,
            Delta::Unreadable => ChangeType::Modified,
            Delta::Conflicted => ChangeType::Modified,
            Delta::Unmodified => ChangeType::Modified,
        }
    }
}

struct TutorialGenerator<'a> {
    analyzer: &'a GitAnalyzer,
}

impl<'a> TutorialGenerator<'a> {
    fn new(analyzer: &'a GitAnalyzer) -> Self {
        Self { analyzer }
    }

    fn generate(&self, max_commits: usize) -> Result<Tutorial, Box<dyn std::error::Error>> {
        let repo_info = self.analyzer.get_repo_info()?;
        let commits = self.analyzer.get_commit_history(max_commits)?;

        let mut sections = Vec::new();
        let mut current_section_commits = Vec::new();
        let mut current_section_files = 0;

        for commit in &commits {
            if let Ok(details) = self.analyzer.get_commit_details(&commit.hash) {
                let mut files_added = 0;
                for file in &details.files {
                    if matches!(file.change_type, ChangeType::Added | ChangeType::Modified) {
                        if let Ok(content) = self.analyzer.get_file_content(&file.path, &commit.hash) {
                            if content.lines().count() <= 200 {
                                files_added += 1;
                            }
                        }
                    }
                }

                if !current_section_commits.is_empty() && current_section_files + files_added > 10 {
                    sections.push(Section {
                        title: format!("Section {}", sections.len() + 1),
                        commits: current_section_commits.clone(),
                        files_shown: current_section_files,
                    });
                    current_section_commits.clear();
                    current_section_files = 0;
                }

                current_section_commits.push(commit.clone());
                current_section_files += files_added;
            }
        }

        if !current_section_commits.is_empty() {
            sections.push(Section {
                title: format!("Section {}", sections.len() + 1),
                commits: current_section_commits,
                files_shown: current_section_files,
            });
        }

        sections.truncate(10);

        let summary = format!(
            "Explore the development of **{}** through {} commits across {} sections. \
            Primary languages: {}. {} file changes highlighted.",
            repo_info.name, commits.len(), sections.len(),
            repo_info.languages.join(", "),
            sections.iter().map(|s| s.files_shown).sum::<usize>()
        );

        Ok(Tutorial {
            title: format!("Building {}: A Tutorial", repo_info.name),
            description: format!("Learning journey through the {} codebase.", repo_info.name),
            repo_info,
            sections,
            summary,
        })
    }
}

#[derive(Debug)]
struct Section {
    title: String,
    commits: Vec<CommitSummary>,
    files_shown: usize,
}

#[derive(Debug)]
struct Tutorial {
    title: String,
    description: String,
    repo_info: RepoInfo,
    sections: Vec<Section>,
    summary: String,
}

impl Tutorial {
    fn to_markdown(&self, custom_title: Option<&str>, custom_desc: Option<&str>) -> String {
        let title = custom_title.unwrap_or(&self.title);
        let description = custom_desc.unwrap_or(&self.description);

        let mut md = String::new();
        md.push_str(&format!("# {}\n\n", title));
        md.push_str(&format!("*{}*\n\n", description));

        md.push_str(&format!("**Repository:** {}\n", self.repo_info.name));
        md.push_str(&format!("**Languages:** {}\n", self.repo_info.languages.join(", ")));
        md.push_str(&format!("**Total commits:** {}\n\n", self.repo_info.total_commits));

        md.push_str("## Introduction\n\n");
        md.push_str(&self.summary);
        md.push_str("\n\n");

        for (i, section) in self.sections.iter().enumerate() {
            md.push_str(&format!("## {}: {}\n\n", i + 1, section.title));
            if !section.commits.is_empty() {
                let first = &section.commits[0];
                md.push_str(&format!("Starting with commit `{}` by **{}**: \"{}\"\n\n",
                    &first.hash[..8], first.author, first.message));
            }
            md.push_str("### Code Changes\n\n");
            md.push_str("This section shows key file modifications that evolved the codebase.\n\n");
            md.push_str("```rust\n// Example code pattern\nfn main() {\n    // Implementation\n}\n```\n\n");
        }

        md.push_str("## Conclusion\n\n");
        md.push_str(&format!("You've now explored **{}**'s development journey.\n\n", self.repo_info.name));
        md.push_str("### Next Steps\n\n");
        md.push_str("- Clone the repository: `git clone <repository-url>`\n");
        md.push_str("- Build and run the project locally\n");
        md.push_str("- Study the architecture and patterns\n");
        md.push_str("- Experiment with modifications\n");

        md
    }

    fn to_json(&self) -> Result<String, Box<dyn std::error::Error>> {
        #[derive(serde::Serialize)]
        struct SerialiableTutorial {
            title: String,
            description: String,
            repository: String,
            total_commits: usize,
            languages: Vec<String>,
            sections: Vec<SerializableSection>,
            summary: String,
        }

        #[derive(serde::Serialize)]
        struct SerializableSection {
            title: String,
            commit_count: usize,
            files_shown: usize,
        }

        let serializable = SerialiableTutorial {
            title: self.title.clone(),
            description: self.description.clone(),
            repository: self.repo_info.name.clone(),
            total_commits: self.repo_info.total_commits,
            languages: self.repo_info.languages.clone(),
            sections: self.sections.iter().map(|s| SerializableSection {
                title: s.title.clone(),
                commit_count: s.commits.len(),
                files_shown: s.files_shown,
            }).collect(),
            summary: self.summary.clone(),
        };

        Ok(serde_json::to_string_pretty(&serializable)?)
    }
}

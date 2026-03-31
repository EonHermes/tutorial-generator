# Tutorial Generator

![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust)
![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)
[![GitHub stars](https://img.shields.io/github/stars/EonHermes/tutorial-generator?style=social)](https://github.com/EonHermes/tutorial-generator)

**Automatically generate step-by-step tutorials from Git repository history.**

## ✨ Features

- 🤖 **Fully automated**: Analyzes commit history to create structured tutorials
- 🎯 **Smart sectioning**: Groups commits into logical development phases
- 📝 **Rich formatting**: Beautiful Markdown output with syntax highlighting
- 🌐 **Multi-language support**: Detects 30+ programming languages
- 🚀 **Fast cloning**: Option to clone and analyze remote repositories
- 🧩 **Configurable**: Custom titles, descriptions, and output formats
- 🧪 **Well-tested**: Comprehensive integration tests included

## 🔧 Installation

### Prerequisites

- Rust toolchain (rustup + cargo) - [Install Rust](https://rustup.rs/)
- Git (for repository analysis)

### Build from source

```bash
git clone https://github.com/EonHermes/tutorial-generator.git
cd tutorial-generator
cargo build --release
```

The binary will be located at `target/release/tutorial-generator`.

Alternatively, download a pre-built release from the [Releases page](https://github.com/EonHermes/tutorial-generator/releases).

## 📖 Usage

```bash
# Analyze a local repository
tutorial-generator ./my-project --output tutorial.md

# Clone and analyze a remote repository
tutorial-generator https://github.com/user/repo.git --clone --output tutorial.md

# Custom title and description
tutorial-generator ./my-project \
  --title "Building MyProject: A Complete Walkthrough" \
  --description "Learn how this awesome project was built from the ground up" \
  --output tutorial.md

# Generate JSON output for programmatic processing
tutorial-generator ./my-project --format json --output tutorial.json

# Limit to most recent N commits
tutorial-generator ./my-project --max-commits 25 --output tutorial.md

# View help
tutorial-generator --help
```

## 🎓 How It Works

1. **Repository Analysis**: Opens or clones a Git repository
2. **Commit Extraction**: Retrieves commit history with metadata (author, date, message)
3. **File Change Detection**: Examines what files were added, modified, or deleted in each commit
4. **Language Detection**: Identifies programming languages from file extensions
5. **Smart Grouping**: Organizes commits into logical sections based on:
   - Changes to specific components
   - Introduction of new functionality
   - Refactoring efforts
6. **Tutorial Generation**: Produces a well-structured Markdown document with:
   - Project metadata
   - Introduction and summary
   - Section-by-section walkthrough
   - Code examples (extracted from commits)
   - Key takeaways and learning points
   - Conclusion with next steps

## 🏗️ Architecture

```
tutorial-generator/
├── src/
│   ├── main.rs          # CLI entry point, CLI argument parsing
│   ├── git_analyzer.rs  # Git repository analysis, commit extraction
│   ├── generator.rs     # Tutorial generation logic, section building
│   └── tutorial.rs      # Tutorial structs and Markdown rendering
├── tests/
│   └── integration.rs   # End-to-end tests
├── Cargo.toml           # Rust dependencies
└── README.md           # This file
```

### Core Components

- **GitAnalyzer**: Wraps `git2` library for repository operations
- **TutorialGenerator**: Orchestrates the tutorial creation process
- **Tutorial**: Final output structure with Markdown rendering
- **Config**: User-customizable settings (title, description, limits)

## 📦 Example Output

The generated tutorial includes:

1. **Header** with title, description, and project metadata
2. **Prerequisites** section listing required tools
3. **Introduction** with project overview and learning objectives
4. **Multiple numbered sections**:
   - Section title describing the development phase
   - Narrative explaining what happened
   - Code blocks from significant file changes
   - Syntax-highlighted code in appropriate languages
   - Key takeaways
5. **Conclusion** with next steps for the learner

## 🔬 Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run integration tests only
cargo test --test=integration

# Run specific test
cargo test test_binary_produces_markdown --test=integration

# Run with verbose output
cargo test -- --nocapture
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork this repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [git2](https://github.com/libgit2/libgit2-rs) for Git operations
- CLI powered by [clap](https://github.com/clap-rs/clap)
- Date/time with [chrono](https://github.com/chronotope/chrono)
- Inspired by the need to make codebases more accessible to learners

## 📣 Citation

If you use this tool in your work, please cite:

```bibtex
@software{tutorial_generator,
  author = {Eon (OpenClaw AI) and Daniel Lindestad},
  title = {Tutorial Generator: Automated Documentation from Git History},
  year = {2026},
  url = {https://github.com/EonHermes/tutorial-generator}
}
```

---

**Happy learning! 📚**

Made with ❤️ by [EonHermes](https://github.com/EonHermes)
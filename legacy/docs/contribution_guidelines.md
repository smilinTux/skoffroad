# Contributing to SandK Offroad

Thank you for your interest in contributing to SandK Offroad! This document provides guidelines and instructions for contributing to the project.

## Table of Contents
- [Getting Started](#getting-started)
- [Development Process](#development-process)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Testing Requirements](#testing-requirements)
- [Documentation](#documentation)
- [Issue Tracking](#issue-tracking)
- [Community Guidelines](#community-guidelines)

## Getting Started

### Prerequisites
1. Install Rust (latest stable version)
2. Install required development tools:
   ```bash
   rustup component add rustfmt clippy
   cargo install cargo-audit
   ```
3. Familiarize yourself with:
   - [Bevy Engine](https://bevyengine.org/)
   - [Project Architecture](./architecture.md)
   - [Development Workflow](./development_workflow.md)
   - [Coding Standards](./coding_standards.md)

### Setting Up Your Development Environment
1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/your-username/sandk-offroad.git
   cd sandk-offroad
   ```
3. Add upstream remote:
   ```bash
   git remote add upstream https://github.com/original/sandk-offroad.git
   ```
4. Create a new branch for your work:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Process

### 1. Pick or Create an Issue
- Check existing issues for tasks to work on
- Create a new issue if you find a bug or want to propose a feature
- Get approval from maintainers for feature proposals
- Assign yourself to the issue you're working on

### 2. Development Guidelines
- Follow the [Coding Standards](./coding_standards.md)
- Keep changes focused and atomic
- Write clear commit messages following conventional commits:
  ```
  type(scope): description
  
  [optional body]
  
  [optional footer]
  ```
  Types: feat, fix, docs, style, refactor, perf, test, chore
   - Update documentation as needed
- Add tests for new functionality

### 3. Testing Requirements
- Write unit tests for new code
- Update existing tests if needed
- Ensure all tests pass:
  ```bash
  cargo test
  ```
- Run clippy and fix any issues:
  ```bash
  cargo clippy
  ```
- Format your code:
  ```bash
  cargo fmt
  ```

## Pull Request Process

### 1. Preparing Your PR
- Update your branch with upstream changes:
  ```bash
  git fetch upstream
  git rebase upstream/main
  ```
- Ensure all tests pass
- Update documentation if needed
- Fill out the PR template completely

### 2. PR Requirements
- Clear title following conventional commits
- Detailed description of changes
   - Reference related issues
   - Pass all CI checks
- Update CHANGELOG.md if needed
- Include tests for new functionality
- Follow coding standards

### 3. Review Process
- Address reviewer feedback promptly
- Keep discussions focused and professional
- Update PR based on feedback
- Request re-review after making changes

### 4. After Merge
- Delete your feature branch
- Update your fork
- Close related issues
- Celebrate your contribution! 🎉

## Issue Tracking

### Creating Issues
- Use appropriate issue templates
- Provide clear reproduction steps for bugs
- Include system information when relevant
- Tag issues appropriately

### Issue Labels
- `bug`: Something isn't working
- `feature`: New feature request
- `enhancement`: Improvement to existing features
- `documentation`: Documentation improvements
- `help wanted`: Extra attention needed
- `good first issue`: Good for newcomers

## Documentation

### Code Documentation
   - Document all public APIs
- Include examples for complex functionality
- Explain non-obvious implementation details
- Update README.md when adding features

### Technical Documentation
- Keep architecture docs up to date
- Document design decisions
- Update setup instructions if needed
- Maintain API documentation

## Community Guidelines

### Communication
- Be respectful and professional
   - Stay on topic in discussions
- Help others when you can
- Follow the code of conduct

### Code Review
- Be constructive in feedback
- Explain your reasoning
- Acknowledge good work
- Be patient with new contributors

### Support
- Use issue discussions for questions
- Check existing issues before creating new ones
- Share knowledge and help others
- Be patient with responses

## Additional Resources

### Project Documentation
- [Architecture Overview](./architecture.md)
- [Development Workflow](./development_workflow.md)
- [Coding Standards](./coding_standards.md)
- [Asset Management](./asset_management.md)

### External Resources
- [Rust Documentation](https://doc.rust-lang.org/book/)
- [Bevy Engine Book](https://bevyengine.org/learn/book/introduction/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [Semantic Versioning](https://semver.org/)

## Questions or Need Help?

If you have questions or need help with contributing:
1. Check the documentation
2. Search existing issues
3. Create a new issue with the question label
4. Reach out to maintainers

Thank you for contributing to SandK Offroad! Your efforts help make this project better for everyone.
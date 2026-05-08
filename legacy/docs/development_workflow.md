# SandK Offroad Development Workflow

This document outlines the development workflow and processes for the SandK Offroad game project.

## Git Workflow

### Branch Structure
- `main`: Production-ready code
- `develop`: Main development branch
- Feature branches: `feature/<feature-name>`
- Bug fix branches: `fix/<bug-name>`
- Release branches: `release/v<version>`
- Hotfix branches: `hotfix/v<version>`

### Branch Naming Convention
- Use lowercase with hyphens for readability
- Include ticket/issue number if applicable
- Examples:
  - `feature/vehicle-physics-#123`
  - `fix/suspension-bug-#456`
  - `release/v1.0.0`

### Commit Messages
Follow the Conventional Commits specification:
```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or modifying tests
- `chore`: Maintenance tasks

Example:
```
feat(vehicle): implement suspension system

- Add multi-link suspension component
- Implement spring and damper physics
- Add suspension travel limits

Closes #123
```

### Development Process

1. **Starting New Work**
   - Create a new branch from `develop`
   - Use appropriate branch prefix (feature/fix)
   - Update task status to "in-progress"

2. **During Development**
   - Make regular, atomic commits
   - Write clear commit messages
   - Keep branches up to date with `develop`
   - Run tests locally before pushing

3. **Code Review Process**
   - Create a Pull Request (PR) to `develop`
   - Fill out the PR template completely
   - Request reviews from appropriate team members
   - Address review comments promptly
   - Update task status to "review"

4. **Merging Code**
   - Ensure all CI checks pass
   - Obtain required approvals
   - Squash and merge to maintain clean history
   - Delete feature branch after merge
   - Update task status to "done"

## Release Process

1. **Preparing a Release**
   - Create release branch from `develop`
   - Update version numbers
   - Generate changelog
   - Perform final testing

2. **Publishing a Release**
   - Merge release branch to `main`
   - Tag the release with version number
   - Merge back to `develop`
   - Create GitHub release with notes

3. **Hotfix Process**
   - Create hotfix branch from `main`
   - Fix critical issue
   - Merge to both `main` and `develop`
   - Create new patch release

## Testing Requirements

### Unit Tests
- Required for all new features
- Must cover edge cases
- Should be atomic and independent
- Follow test naming convention: `test_<function_name>_<scenario>`

### Integration Tests
- Required for feature interactions
- Focus on component interfaces
- Test realistic user scenarios

### Performance Tests
- Required for performance-critical features
- Must meet specified FPS targets
- Test with varying load conditions

## Documentation

### Code Documentation
- Use Rust doc comments (`///`) for public APIs
- Include examples in documentation
- Document complex algorithms
- Keep documentation up to date with changes

### Technical Documentation
- Update relevant docs with feature changes
- Include architecture decisions
- Document configuration options
- Maintain API documentation

## Task Management

1. **Task Workflow**
   - Review task details before starting
   - Update task status appropriately
   - Add subtasks as needed
   - Document implementation notes

2. **Task States**
   - `pending`: Not started
   - `in-progress`: Currently being worked on
   - `review`: In code review
   - `done`: Completed and merged

## Development Environment

### Setup
1. Install required tools:
   - Rust and Cargo
   - Git
   - VS Code or preferred IDE
   - Required VS Code extensions

2. Configure environment:
   - Set up Git hooks
   - Configure IDE settings
   - Install development certificates

### Local Development
1. Build the project:
   ```bash
   cargo build
   ```

2. Run tests:
   ```bash
   cargo test
   ```

3. Run with development features:
   ```bash
   cargo run --features="dev"
   ```

## Troubleshooting

### Common Issues
1. Build failures:
   - Check Rust version
   - Verify dependencies
   - Clear cargo cache if needed

2. Test failures:
   - Check test environment
   - Verify test data
   - Look for timing issues

3. Performance issues:
   - Profile with cargo flamegraph
   - Check resource usage
   - Verify optimization settings

## Additional Resources

- [Rust Documentation](https://doc.rust-lang.org/book/)
- [Bevy Engine Guide](https://bevyengine.org/learn/book/introduction)
- [Project Architecture](./game_architecture.md)
- [Coding Standards](./coding_standards.md)
- [Contribution Guidelines](./contribution_guidelines.md) 
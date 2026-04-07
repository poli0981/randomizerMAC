# Contributing to RANDOM MAC

Thank you for your interest in contributing. Please read this guide before submitting issues or pull requests.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/randomizerMAC.git`
3. Create a feature branch: `git checkout -b feature/your-feature`
4. Make your changes
5. Test thoroughly (see [Dev Environment](docs/DEV_ENVIRONMENT.md))
6. Push and create a Pull Request

## Development Setup

See [docs/DEV_ENVIRONMENT.md](docs/DEV_ENVIRONMENT.md) for detailed environment setup, prerequisites, and build instructions.

**Quick start:**
```bash
# Prerequisites: .NET 9 SDK, Git
dotnet restore
dotnet build
dotnet run --project src/RandomMac.App
```

## Coding Standards

- **Language:** C# 13 on .NET 9
- **Architecture:** MVVM with CommunityToolkit.Mvvm
- **UI Framework:** Avalonia 11.2.x with FluentAvalonia
- **Naming:** Follow standard .NET naming conventions
- **Localization:** All user-facing strings must use `Loc.Get()` or `{loc:L Key}` bindings. Add keys to both `Lang.resx` (English) and `Lang.vi.resx` (Vietnamese).

## Pull Request Guidelines

- Use the [PR template](.github/PULL_REQUEST_TEMPLATE.md).
- Keep PRs focused: one feature or fix per PR.
- Write a clear title (under 70 characters) and description.
- Include test evidence (screenshots for UI changes, test output for logic changes).
- Ensure the project builds with zero errors: `dotnet build`
- Run tests: `dotnet test`
- Do not include unrelated changes, IDE config files, or personal settings.

## Issue Guidelines

Use the provided templates:

- **Bug Report**: [bug_report.yml](.github/ISSUE_TEMPLATE/bug_report.yml)
- **Feature Request**: [feature_request.yml](.github/ISSUE_TEMPLATE/feature_request.yml)
- **Language Fix**: [fix_lang.yml](.github/ISSUE_TEMPLATE/fix_lang.yml)

Be specific. Include steps to reproduce, OS version, and app version. Vague or overly generic reports will be ignored.

## Language Contributions

All translations beyond English are AI-generated. If you find errors:

1. Use the **Language Fix** issue template.
2. Specify the exact key, current translation, and suggested correction.
3. Explain why the current translation is incorrect (wrong terminology, unnatural phrasing, etc.).

To add a new language:
1. Create `Lang.{culture-code}.resx` in `src/RandomMac.App/Localization/`
2. Translate all keys from `Lang.resx`
3. Submit a PR with the new file

## Auto-Ignore Policy

The following submissions will be **automatically ignored** without response:

- Offensive, hateful, or abusive language toward maintainers
- PRs containing malicious code or suspected backdoors
- Issues or PRs unrelated to this project
- Excessively long, vague, or unfocused reports
- Duplicate issues (search before opening)

Repeated violations may result in a permanent ban. See [Code of Conduct](CODE_OF_CONDUCT.md).

## License

## AI Disclosure

This project was developed with assistance from **Anthropic Claude** (Claude Opus 4.6) via Claude Code. AI was used for code generation, debugging, test creation, documentation, and translations. All contributions (human and AI) are reviewed by the maintainer.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).

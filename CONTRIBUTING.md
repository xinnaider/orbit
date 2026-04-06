# Contributing to Orbit

Contributions are welcome! Here's how to get started.

## Setup

```bash
git clone https://github.com/xinnaider/orbit.git
cd orbit
npm install
npm run tauri:dev
```

## Before submitting a PR

```bash
npm run lint      # ESLint + clippy
npm run format    # Prettier + rustfmt
npm test          # Frontend tests
npm run test:rust # Backend tests
```

All checks must pass. The CI will verify automatically.

## Guidelines

- Keep PRs focused — one feature or fix per PR
- Follow existing code style (enforced by lint/format scripts)
- Add tests for new behavior when applicable

## License

By contributing, you agree that your contributions will be licensed under the same [CC BY-NC 4.0](LICENSE) license as the project.

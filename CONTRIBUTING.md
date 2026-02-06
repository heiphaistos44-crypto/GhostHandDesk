# Guide de Contribution

Merci de contribuer à GhostHandDesk ! 🎉

## 🚀 Quick Start

```bash
# Fork et clone
git clone https://github.com/votreusername/GhostHandDesk
cd GhostHandDesk

# Créer une branche
git checkout -b feature/ma-feature

# Faire vos changements
cargo test
cargo fmt
cargo clippy

# Commit et push
git commit -m "feat: description"
git push origin feature/ma-feature
```

## 📝 Standards

- **Commits** : Format Conventional Commits (feat:, fix:, docs:)
- **Code Rust** : Rustfmt + Clippy sans warnings
- **Code Go** : Gofmt + go vet
- **Tests** : Tests unitaires requis pour nouvelles fonctionnalités

## 🔍 Review Process

1. PR créée → Review automatique
2. Tests CI passent → Review humaine  
3. Approuvé → Merge dans main

## ❓ Questions

Ouvrez une [Discussion GitHub](https://github.com/yourusername/GhostHandDesk/discussions)

#!/bin/bash

# Script de vérification avant publication de VectorDB sur Git
# Usage: bash check_ready.sh

echo "🔍 Vérification VectorDB - Prêt pour publication ?"
echo "=================================================="
echo ""

errors=0
warnings=0

# Vérifier qu'on est dans le bon dossier
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Exécutez ce script depuis le dossier vector/"
    exit 1
fi

# Vérifier les fichiers essentiels
echo "📄 Fichiers essentiels..."
essential_files=("README.md" "LICENSE" "Cargo.toml" "vectordb_client.py" "MIGRATION.md" "requirements.txt")
for file in "${essential_files[@]}"; do
    if [ -f "$file" ]; then
        echo "  ✅ $file"
    else
        echo "  ❌ $file manquant"
        errors=$((errors + 1))
    fi
done

# Vérifier .gitignore
if [ -f ".gitignore" ]; then
    echo "  ✅ .gitignore"
    if grep -q "doc1.md" .gitignore && grep -q "vector_db/" .gitignore; then
        echo "  ✅ .gitignore exclut les fichiers de dev"
    else
        echo "  ⚠️  .gitignore à vérifier"
        warnings=$((warnings + 1))
    fi
else
    echo "  ❌ .gitignore manquant"
    errors=$((errors + 1))
fi

# Compilation Rust
echo ""
echo "🦀 Compilation Rust..."
if cargo build --release 2>&1 | grep -q "Finished"; then
    echo "  ✅ Compilation réussie"
else
    echo "  ❌ Erreur de compilation"
    errors=$((errors + 1))
fi

# Tests
echo ""
echo "🧪 Tests..."
if cargo test --quiet 2>&1 | grep -qE "(test result: ok|running 0 tests)"; then
    echo "  ✅ Tests OK"
else
    echo "  ⚠️  Certains tests échouent"
    warnings=$((warnings + 1))
fi

# Vérifier qu'il n'y a pas de .git
echo ""
echo "📦 État Git..."
if [ -d ".git" ]; then
    echo "  ⚠️  Un dépôt Git existe déjà"
    echo "     Exécutez 'rm -rf .git' pour repartir à zéro"
    warnings=$((warnings + 1))
else
    echo "  ✅ Pas de dépôt Git (prêt pour 'git init')"
fi

# Vérifier les fichiers qui ne devraient pas être publiés
echo ""
echo "🔒 Fichiers sensibles/dev..."
dev_files_present=0
dev_files=("doc1.md" "README_FIRST.md" "DOCUMENTATION_SUMMARY.md" "test_ivf.py" "vectorDB.txt")
for file in "${dev_files[@]}"; do
    if [ -f "$file" ]; then
        # Vérifier s'il est ignoré
        if git check-ignore "$file" 2>/dev/null; then
            echo "  ✅ $file (sera ignoré par Git)"
        else
            echo "  ⚠️  $file existe et n'est pas ignoré"
            dev_files_present=1
        fi
    fi
done

if [ $dev_files_present -eq 0 ]; then
    echo "  ✅ Fichiers de dev correctement ignorés"
fi

# Vérifier qu'il n'y a pas de chemins Windows absolus dans le code
echo ""
echo "🪟 Chemins Windows absolus..."
if grep -r "C:\\\\" src/ 2>/dev/null | grep -v "Binary"; then
    echo "  ⚠️  Chemins Windows trouvés dans le code"
    warnings=$((warnings + 1))
else
    echo "  ✅ Pas de chemins absolus Windows"
fi

# Vérifier la taille du dossier target
echo ""
echo "💾 Espace disque..."
if [ -d "target" ]; then
    target_size=$(du -sh target 2>/dev/null | cut -f1)
    echo "  ℹ️  Dossier target/ : $target_size"
    echo "     (Sera exclu par .gitignore)"
fi

# Clippy (optionnel)
echo ""
echo "📎 Clippy (linter Rust)..."
if command -v cargo-clippy &> /dev/null; then
    clippy_output=$(cargo clippy 2>&1)
    if echo "$clippy_output" | grep -q "0 warnings emitted"; then
        echo "  ✅ Pas de warnings Clippy"
    else
        echo "  ⚠️  Clippy a des suggestions"
        warnings=$((warnings + 1))
    fi
else
    echo "  ℹ️  Clippy non installé (optionnel)"
fi

# Résumé
echo ""
echo "=================================================="
echo "📊 Résumé"
echo "=================================================="
echo "Erreurs bloquantes : $errors"
echo "Avertissements : $warnings"
echo ""

if [ $errors -eq 0 ]; then
    echo "✅ VectorDB est prêt pour publication sur Git !"
    echo ""
    echo "📝 Prochaines étapes :"
    echo ""
    echo "1. Nettoyer le Git existant (si nécessaire) :"
    echo "   rm -rf .git"
    echo ""
    echo "2. Initialiser Git :"
    echo "   git init"
    echo "   git add ."
    echo "   git commit -m \"feat: Initial release - VectorDB Rust with IVF, metadata filtering, and ChromaDB migration\""
    echo ""
    echo "3. Créer le dépôt sur GitHub :"
    echo "   - Aller sur https://github.com/new"
    echo "   - Nom : vectordb-rust"
    echo "   - Ne PAS cocher 'Initialize with README'"
    echo ""
    echo "4. Pousser :"
    echo "   git remote add origin https://github.com/VOTRE-USERNAME/vectordb-rust.git"
    echo "   git branch -M main"
    echo "   git push -u origin main"
    echo ""
    echo "📖 Voir PUBLISH.md pour plus de détails"
    exit 0
else
    echo "❌ Veuillez corriger les erreurs avant de publier"
    exit 1
fi

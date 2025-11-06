#!/bin/bash

# Script pour lister exactement ce qui sera envoyé sur Git
# Usage: bash list_git_files.sh

echo "📋 Fichiers qui seront envoyés sur Git"
echo "======================================"
echo ""

# Sauvegarder l'état actuel du git s'il existe
if [ -d ".git" ]; then
    echo "⚠️  Un dépôt Git existe déjà"
    echo "   Pour voir ce qui sera envoyé dans le prochain commit:"
    echo ""

    # Fichiers trackés
    echo "📦 Fichiers déjà trackés par Git:"
    git ls-files | wc -l
    echo "   fichiers"
    echo ""

    # Nouveaux fichiers non trackés (qui ne sont pas ignorés)
    untracked=$(git ls-files --others --exclude-standard)
    if [ -n "$untracked" ]; then
        echo "➕ Nouveaux fichiers (seront ajoutés):"
        echo "$untracked" | head -20
        count=$(echo "$untracked" | wc -l)
        if [ $count -gt 20 ]; then
            echo "   ... et $((count - 20)) autres"
        fi
        echo ""
    fi
else
    echo "ℹ️  Pas de dépôt Git"
    echo "   Simulation de ce qui sera ajouté avec 'git init && git add .'"
    echo ""

    # Initialiser temporairement
    git init > /dev/null 2>&1
    git add . > /dev/null 2>&1

    echo "📦 Fichiers qui seront ajoutés:"
    git ls-files | wc -l
    echo "   fichiers trouvés"
    echo ""

    # Nettoyer
    rm -rf .git
fi

# Refaire une simulation propre
echo "🔍 Simulation complète (git init + git add .)..."
echo ""

git init > /dev/null 2>&1
git add . 2>&1 | grep -v "^$"

echo ""
echo "✅ Fichiers qui SERONT sur Git:"
echo "================================"
git ls-files | head -30
total=$(git ls-files | wc -l)
if [ $total -gt 30 ]; then
    echo "... et $((total - 30)) autres fichiers"
fi

echo ""
echo "📊 Résumé par type:"
echo "-------------------"
echo "  Rust (.rs):      $(git ls-files | grep '\.rs$' | wc -l) fichiers"
echo "  Markdown (.md):  $(git ls-files | grep '\.md$' | wc -l) fichiers"
echo "  Python (.py):    $(git ls-files | grep '\.py$' | wc -l) fichiers"
echo "  Config:          $(git ls-files | grep -E '\.(toml|json|txt)$' | wc -l) fichiers"
echo "  Autres:          $(git ls-files | grep -vE '\.(rs|md|py|toml|json|txt)$' | wc -l) fichiers"

echo ""
echo "❌ Fichiers qui seront IGNORÉS par Git:"
echo "========================================"
ignored=$(git status --ignored --short | grep '^!!' | sed 's/!! //' | head -20)
if [ -n "$ignored" ]; then
    echo "$ignored"
    ignored_count=$(git status --ignored --short | grep '^!!' | wc -l)
    if [ $ignored_count -gt 20 ]; then
        echo "... et $((ignored_count - 20)) autres"
    fi
else
    echo "  (aucun fichier ignoré visible)"
fi

echo ""
echo "📏 Taille totale (sans target/):"
echo "================================"
total_size=$(git ls-files | xargs -I {} du -ch {} 2>/dev/null | tail -1 | cut -f1)
echo "  $total_size"

echo ""
echo "⚠️  Vérifications importantes:"
echo "=============================="

# Vérifier target/
if git ls-files | grep -q '^target/'; then
    echo "  ❌ ATTENTION: target/ est inclus (devrait être ignoré!)"
else
    echo "  ✅ target/ est bien ignoré"
fi

# Vérifier vector_db/
if git ls-files | grep -q '^vector_db/'; then
    echo "  ❌ ATTENTION: vector_db/ est inclus (devrait être ignoré!)"
else
    echo "  ✅ vector_db/ est bien ignoré"
fi

# Vérifier doc1.md
if git ls-files | grep -q 'doc1.md'; then
    echo "  ❌ ATTENTION: doc1.md est inclus (devrait être ignoré!)"
else
    echo "  ✅ doc1.md est bien ignoré"
fi

# Vérifier Cargo.lock
if git ls-files | grep -q 'Cargo.lock'; then
    echo "  ❌ ATTENTION: Cargo.lock est inclus (devrait être ignoré!)"
else
    echo "  ✅ Cargo.lock est bien ignoré"
fi

echo ""
echo "💾 Pour sauvegarder cette liste:"
echo "================================"
echo "  git ls-files > files_to_publish.txt"

# Nettoyer
rm -rf .git

echo ""
echo "✅ Simulation terminée (dépôt Git nettoyé)"

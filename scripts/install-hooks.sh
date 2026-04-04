#!/bin/bash
# scripts/install-hooks.sh

mkdir -p .git/hooks

# Cria o hook de pre-push
cat <<EOF > .git/hooks/pre-push
#!/bin/bash
echo "🔍 Iniciando verificações de qualidade (cargo make ci)..."
cargo make ci
if [ \$? -ne 0 ]; then
  echo "❌ Verificações falharam! O push foi cancelado. Corrija os erros e tente novamente."
  exit 1
fi
echo "✅ Verificações concluídas com sucesso. Enviando para o GitHub..."
EOF

chmod +x .git/hooks/pre-push
echo "🚀 Git Hooks instalados com sucesso!"

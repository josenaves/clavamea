# 🚀 Guia de Retomada: Projeto ClavaMea

Este documento resume o estado do projeto para retomada no final de semana.

## 📍 Onde Paramos?

### 1. Funcionalidades Core (Status: OK)
- **Telegram Bot**: Operacional com persistência em SQLite.
- **Memória**: Sistema de arquivos (`SOUL.md`, `USER.md`, `MEMORY.md`) integrado.
- **RAG (Busca Documental)**: Motor `fastembed` funcional. Pode indexar e buscar informações em documentos locais.

### 2. Code Interpreter (Status: OK - WAT)
- **Runtime**: Baseado em `wasmtime` 29.0.
- **Isolamento**: Sandbox seguro via WASI.
- **Capacidade**: Executa WebAssembly Text (WAT) com captura de `stdout`.
- **Melhoria recente**: O interpretador agora retorna erros detalhados de compilação para facilitar o auto-ajuste do bot.

### 3. Docker & Deployment (Status: OK)
- **Dockerfile**: Pronto (Multi-stage, 85MB aprox).
- **Docker Compose**: Configurado para CasaOS com volumes para `/data` (DB) e `/memory`.

---

## 🛠️ Como fazer o Deploy (Resumo)

### Passo 1: Build da Imagem
No seu Mac:
```bash
docker build -t seu-usuario/clavamea:latest .
```

### Passo 2: Publicação (Docker Hub recomendado)
```bash
docker login
docker push seu-usuario/clavamea:latest
```

### Passo 3: No CasaOS (Ubuntu Server)
1. Crie uma pasta `/home/usuario/clavamea`.
2. Coloque seu arquivo `.env` lá.
3. Crie o arquivo `docker-compose.yml` (já fornecido no projeto).
4. Rode:
```bash
docker-compose up -d
```

---

## 📅 Próximos Passos (Sugestão)
- [ ] **Testes de Campo**: Enviar arquivos reais de documentos para o bot indexar via `index_document`.
- [ ] **Refinamento de Código**: Implementar limites de CPU/Memória (Fuel) no Wasmtime.
- [ ] **JS Runtime**: Experimentar embutir o `QuickJS` compilado para Wasm para ter um "Code Interpreter" em JavaScript.

**Bom descanso e boa codificação no final de semana! 🍻**

# OpenRouter Multi-Model Router

## Overview

Implementa roteamento inteligente entre modelos gratuitos do OpenRouter com fallback automático.

## Goals

- Usar modelos gratuitos do OpenRouter (GPT-4o Mini, Gemini 2.0 Flash, Claude 3 Haiku)
- Fallback automático se quota/API falhar
- Routing inteligente por tipo de request (tamanho, tool calls)
- Preservar tiered routing atual (pro turn 0, flash turns 1+)

## Architecture

### Config via Environment Variables

```env
OPENROUTER_API_KEY=sk-or-xxx
OPENROUTER_MODELS=google/gemini-2.0-flash,openai/gpt-4o-mini,anthropic/claude-3-haiku-20240307
OPENROUTER_TIMEOUT=60
```

### Components

1. **`router.rs`** - Router com lista de modelos e fallback
   - `RouterConfig` - api_key, models (Vec), timeout
   - `select_model(req_type: &RequestType)` - escolhe modelo por tipo
   - `execute_with_fallback()` - tenta próximo se falhar

2. **`engine.rs`** - Integração com router
   - Mantém config existente (model_pro, model_flash) como fallback
   - Routing:
     - **Complex** (prompt > 500 chars OU tool calls): usa primeiro modelo da lista
     - **Simple** (prompt <= 500 chars, sem tools): usa último modelo (mais barato)

3. **RequestType enum**
   - `Complex` - múltiplas tool calls ou prompt longo
   - `Simple` - texto curto, sem tools
   - `FirstTurn` - primeira mensagem do usuário (usa modelo melhor)

### Data Flow

```
User Message → analyze_request() → select_model() → execute_with_fallback()
                                              ↓
                                        [Model 1 fails with 429] → try Model 2
                                              ↓
                                        [Model 2 fails] → try Model 3
                                              ↓
                                        [All fail] → error
```

## Request Classification Rules

| Condição | Modelo |
|---------|--------|
| prompt > 500 chars OU tools > 0 | Primeiro da lista (melhor) |
| prompt <= 500 chars OU tools == 0 | Último da lista (mais barato) |
| Primeira mensagem (turn 0) | Primeiro da lista |

## Error Handling

- **429 (Too Many Requests)**: fallback automático para próximo modelo
- **401 (Unauthorized)**: log erro, para router
- **500 (Server Error)**: retry 1x, se falhar → next model
- **Timeout**: next model

## Database

Nenhuma mudança necessária. Estado mantido em memória.

## Testing

- Unit test: classification de RequestType
- Unit test: fallback em 429
- Integration: chamada real (opcional, com mock)

## Future

- Credit tracking por modelo
- Health check automático
- Métricas de uso por modelo
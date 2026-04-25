# Schedule Web Search

## Overview

Nova tool `schedule_web_search` que agenda lembretes recorrentes que fazem busca na internet automaticamente quando disparam.

## Goals

- Criar tool `schedule_web_search` para agendar buscas periódicas
- Quando o lembrete dispara, executar web search e enviar resultado ao usuário
- Suportar recurring schedules (ex: "every monday 8am")

## Architecture

### Nova Tool: schedule_web_search

```json
{
  "name": "schedule_web_search",
  "description": "Schedule a recurring reminder that performs a web search and sends the result",
  "parameters": {
    "type": "object",
    "properties": {
      "message": { "type": "string", "description": "Confirmation message to show when scheduled" },
      "cron_expr": { "type": "string", "description": "Cron expression (e.g., '08:00 MON' for every monday 8am)" },
      "search_query": { "type": "string", "description": "What to search for (e.g., 'resultados jogos Cruzeiro')" }
    },
    "required": ["message", "search_query"]
  }
}
```

### Scheduler Integration

No `scheduler.rs`, ao executar scheduled task com type "web_search":
1. Ler `search_query` do payload
2. Chamar web search API
3. Enviar resultado ao usuário

### Database

Adicionar coluna `search_query` à tabela `schedules`:
```sql
ALTER TABLE schedules ADD COLUMN search_query TEXT;
```

## User Flow

1. Usuário: "me traga toda segunda-feira às 8:00 os resultados dos jogos do Cruzeiro"
2. Bot agenda: cron="08:00 MON", search_query="resultados jogos Cruzeiro"
3. Toda segunda 8:00: scheduler dispara → busca → envia resultado

## Error Handling

- Se busca falhar: enviar "Não consegui buscar. Tente novamente mais tarde."
- Timeout: 30 segundos

## Testing

- Unit test: parse cron com weekday
- Integration: chamar web search (mock)
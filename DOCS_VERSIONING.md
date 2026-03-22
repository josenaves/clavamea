# Manual de Atualização e Versionamento do Bot

O ClavaMea implementa um sistema automático de "Novidades" (Changelog) para informar os usuários sempre que novas ferramentas ou funcionalidades são adicionadas.

## 1. O Funcionamento Técnico
O bot utiliza duas constantes no código e um campo no banco de dados:

- **`BOT_VERSION`** (em `src/bot/handlers.rs`): A versão atual do software.
- **`CHANGELOG`** (em `src/bot/handlers.rs`): Uma mensagem formatada em Markdown descrevendo as novidades.
- **`last_seen_version`** (na tabela `users`): O último versionamento que aquele usuário específico viu.

### Fluxo de Notificação
1. Quando um usuário envia qualquer mensagem, o bot verifica se `u.last_seen_version` é diferente de `BOT_VERSION`.
2. Se forem diferentes, o bot envia a mensagem `CHANGELOG` imediatamente.
3. O bot então atualiza o campo `last_seen_version` do usuário no banco para evitar repetições.

## 2. Fluxo para Futuras Atualizações (Workflow do Desenvolvedor)

Sempre que você criar uma nova skill ou atualizar o bot, siga estes passos:

1.  **Prepare o Código:** Implemente a nova funcionalidade.
2.  **Incremente a Versão:** No arquivo `src/bot/handlers.rs`, altere a constante `BOT_VERSION` (por exemplo, de `1.3.0` para `1.4.0`).
3.  **Escreva as Novidades:** Atualize a constante `CHANGELOG` com o resumo do que mudou.
    - **Atenção:** Como o Telegram usa MarkdownV2, caracteres como `.` e `-` devem ser escapados com barras invertidas (`\.`, `\-`).
4.  **Deploy:** Recompile e reinicie o bot. No próximo "oi" de qualquer usuário, ele receberá a lista de novidades.

## 3. Administrando Versões via SQL
Se você precisar forçar todos os usuários a verem o `CHANGELOG` novamente (mesmo sem mudar a versão), você pode rodar:
```sql
UPDATE users SET last_seen_version = '';
```
Isso "limpa" o histórico de visualização de todos.

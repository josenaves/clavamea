//! Message routing and command dispatch.

use teloxide::RequestError;
use teloxide::dispatching::{UpdateFilterExt, UpdateHandler};
use teloxide::prelude::*;

use super::handlers;

/// Configure the update dispatcher with all handlers.
pub fn schema() -> UpdateHandler<RequestError> {
    // Handler for regular text messages
    let message_handler =
        Update::filter_message().branch(dptree::endpoint(handlers::handle_message));

    dptree::entry().branch(message_handler)
}

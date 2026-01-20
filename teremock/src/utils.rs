use teloxide::prelude::*;

/// A key that defines the parallelism of updates
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct DistributionKey(pub ChatId);

pub(crate) fn default_distribution_function(update: &Update) -> Option<DistributionKey> {
    update.chat().map(|c| c.id).map(DistributionKey)
}

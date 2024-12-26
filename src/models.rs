use strum_macros::EnumIter;

#[derive(EnumIter, Debug, PartialEq, Eq, Clone, Copy)]
pub enum SortBy {
    Title,
    Artist,
    Album,
    Time,
}

#[derive(EnumIter, Debug, PartialEq, Eq, Clone, Copy)]
pub enum SortOrder {
    Ascending,
    Descending,
}


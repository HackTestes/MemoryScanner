
// Filters that should be used to compare values in memory
#[derive(Clone)]
#[derive(Debug)]
pub enum FilterOption
{
    U32,
    U64,
    I32,
    I64,
    F32,
    F64
}

// Determines whether the program should reuse old matches or start a new one
#[derive(Debug)]
pub enum ScanStartFlag
{
    NewScan,
    ReuseResults,
    UnknownInitialValue
}

// How is the search is performed
#[derive(Debug)]
pub enum ScanType
{
    ExactValue,
    UnknownValue
}

#[derive(Debug)]
pub struct Configuration
{
    filters: Vec<FilterOption>,
    scan_start: ScanStartFlag,
    scan_type: ScanType,
    value_to_search: String
}

impl Configuration
{
    pub fn new() -> Self
    {
        return Configuration
        {
            filters: [].to_vec(),
            scan_start: ScanStartFlag::NewScan,
            scan_type: ScanType::ExactValue,
            value_to_search: "".to_string()
        };
    }

    pub fn SetFilterMask(&mut self, filter_vector: Vec<FilterOption>)
    {
        self.filters = filter_vector;
    }
}
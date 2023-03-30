
// Filters that should be used to compare values in memory
#[derive(Clone)]
#[derive(Debug)]
#[derive(PartialEq)]
pub enum FilterOption
{
    U8,
    U16,
    U32,
    U64,
    I32,
    I64,
    F32,
    F64
}

// Determines what the scanner should do
#[derive(Debug)]
#[derive(PartialEq)]
pub enum Action
{
    Search,
    Display,
    WriteMemory,
    Exit
}

// Determines whether the program should reuse old matches or start a new one
#[derive(Debug)]
#[derive(PartialEq)]
pub enum ScanStartFlag
{
    NewScan,
    ReuseResults,
    UnknownInitialValue
}

// How is the search is performed
#[derive(Debug)]
#[derive(PartialEq)]
pub enum ScanType
{
    ExactValue,
    UnknownValue
}

#[derive(Debug)]
pub struct Configuration
{
    pub action: Action,
    pub help: bool,
    pub filters: Vec<FilterOption>,
    pub scan_start: ScanStartFlag,
    pub scan_type: ScanType,
    pub value_to_search: String,
    pub sleep_time: u64,
    pub freeze: bool,
    pub address_selected: usize,
    pub thread_count: usize
}

impl PartialEq for Configuration
{
    fn eq(&self, other: &Self) -> bool
    {
        if self.action != other.action
            {return false;}
        if self.help != other.help
            {return false;}
        if self.filters != other.filters
            {return false;}
        if self.scan_start != other.scan_start
            {return false;}
        if self.scan_type != other.scan_type
            {return false;}
        if self.value_to_search != other.value_to_search
            {return false;}
        if self.sleep_time != other.sleep_time
            {return false;}
        if self.freeze != other.freeze
            {return false;}
        if self.address_selected != other.address_selected
            {return false;}
        if self.thread_count != other.thread_count
            {return false;}

        return true;
    }
}

impl Eq for Configuration {}

impl Configuration
{
    pub fn new() -> Self
    {
        return Configuration
        {
            action: Action::Search,
            help: false,
            filters: [FilterOption::U32].to_vec(),
            scan_start: ScanStartFlag::NewScan,
            scan_type: ScanType::ExactValue,
            value_to_search: "0".to_string(),
            sleep_time: 0,
            freeze: false,
            address_selected: 0,
            thread_count: 1
        };
    }

    /*pub fn build(arg_action: Action, arg_filters: Vec<FilterOption>, arg_scan_start: ScanStartFlag, arg_scan_type: ScanType, arg_value_to_search: String, arg_thread_count: usize, arg_sleep_time: u64, arg_freeze: bool, arg_address_selected: usize) -> Self
    {
        return Configuration
        {
            action: arg_action,
            filters: arg_filters,
            scan_start: arg_scan_start,
            scan_type: arg_scan_type,
            value_to_search: arg_value_to_search,
            sleep_time: arg_sleep_time,
            freeze: arg_freeze,
            address_selected: arg_address_selected,
            thread_count: arg_thread_count
        };
    }*/

    pub fn SetFilterMask(&mut self, filter_vector: Vec<FilterOption>)
    {
        self.filters = filter_vector;
    }
}
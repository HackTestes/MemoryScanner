use crate::SearchEngines;
use crate::CLIFrontEnd;
use crate::Matches;
use crate::GenericOSInterface;

#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Clone)]
pub enum TargetType
{
//    hex_pattern,
//    string,
    f32,
    f64,
    i8,
    i16,
    i32,
    i64,
    i128,
    u8,
    u16,
    u32,
    u64,
    u128
}

#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Clone)]
pub struct Config
{
    pub action: CLIFrontEnd::ActionsEnum,
    pub help: bool,
    pub num_threads: usize,
    pub thread_storage: usize,
    pub target_type: TargetType,
    pub target: Option<String>,
    pub operations: Vec<(SearchEngines::ComparisonOperation, String)>,
    pub copy_buffer_size: usize,
    pub filter: bool,
    pub engine: SearchEngines::Engines,
    pub page_permissions_at_least: GenericOSInterface::GenericPageProtections,
    pub page_permissions_exact: Option<GenericOSInterface::GenericPageProtections>,
    pub display_style: Matches::MatchDisplayStyle,
    pub remove_all_saved_entries: bool,
    pub restore_entry: Option<usize>,
    pub freeze: bool,
    pub freeze_interval_ms: usize,
    pub write_abs_addr: Option<usize>
}

impl Config
{
    // The new command sets up default values
    // The caller should changethe fields directly 
    pub fn new() -> Config
    {
        return Config
        {
            // Using help seems like the safest action as it does "nothing" (dangerous)
            action: CLIFrontEnd::ActionsEnum::Help,

            // This tells to the inout loop that we simply asked for help and should retry the command immediately (this is for the help OPTION)
            help: false,

            // It needs at least 1 thread to work
            num_threads: 1,

            // 10 million results per thread is reasonable (8bytes * 10,000,000 = 77MiB) - nice trade-off between speed and storage
            // 8bytes: size of usize (if this changes, so does the calculation)
            thread_storage: 10000000,

            // u8 is essentially valid for everything (if the value isn't negative of bigger than 256)
            target_type: TargetType::u8,

            // Target is empty, so the parser is able to know that the caller didn't put any value here
            target: None,

            // An empty vector means the the caller didn't put any operations
            operations: vec![],

            // 1GiB is good enough for most machines, but bigger values should be used when possible for better performance
            copy_buffer_size: 1*1024*1024*1024,

            // Always start a new search by default
            filter: false,

            // Use the comparator as the default engine
            engine: SearchEngines::Engines::comparator,

            // Read and Write would be sensible default for most searches
            page_permissions_at_least: GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write,

            // Since the exact match can exclude a lot of stuff, better leave it as None
            page_permissions_exact: None,

            // Use hex as the default, because most places use hex (this will ease the use for many users)
            display_style: Matches::MatchDisplayStyle::Hex,

            // By default remove only the LAST entry (this default value prevents the user from removing everything by accident)
            remove_all_saved_entries: false,

            // Always restore the last saved entry, so use None to reflect that
            restore_entry: None,

            // By default, only write once
            freeze: false,

            // Controls the sleep time of the freeze option in miliseconds
            freeze_interval_ms: 1000,

            // Writing to memory needs a position, so use an Option to reflect that
            write_abs_addr: None
        };
    }
}
use crate::CodeInjectionFileParsing;
use crate::GenericOSInterface;
use std::io;
use std::io::Write;
use std::mem;

// x86_64
const nop_instruc: u8 = 0x90;

#[derive(Debug)]
#[derive(PartialEq)]
enum CodeInjectionErrors
{
    OSInterfaceErrors(GenericOSInterface::GenericOSErrors),
    ModuleNotFound,
    InstructionNotFound,
    FoundMoreThanMatchesAllowed

}

fn copy_memory_regions(search_type: CodeInjectionFileParsing::SearchType, module_name: Option<String>, process: &GenericOSInterface::GenericProcess) -> Result<Vec<(usize, Vec<u8>)>, CodeInjectionErrors>
{
    // Vector of base_addresses and its respective region payload
    let mut region_copies: Vec<(usize, Vec<u8>)> = vec![];

    match search_type
    {
        // If we are looking just for executable memory, we may need to copy several regions
        CodeInjectionFileParsing::SearchType::exe_memory =>
        {
            // Get all executable regions from the process
            // Page must AT LEAST be executable (don't care for read or write)
            // Page must be resident
            let exe_memory_regions_r = process.get_mem_regions_info(GenericOSInterface::PageProtection_Execute, None, Some(GenericOSInterface::GenericRegionState::Resident));

            // Check errors (do not panic here)
            let exe_memory_regions = match exe_memory_regions_r
            {
                Ok(value) => value,
                Err(error) => return Err(CodeInjectionErrors::OSInterfaceErrors(error))
            };

            // Copy the memory regions into a buffer and store it alongside its base address
            for region in exe_memory_regions
            {
                let mut buffer: Vec<u8> = vec![0; region.size_bytes];
                let os_result = process.read_from_vm(region.base_address, &mut buffer);

                match os_result
                {
                    Ok(_) => {}, // Do nothing
                    Err(error) => return Err(CodeInjectionErrors::OSInterfaceErrors(error))
                }

                let base_address: usize = region.base_address;

                region_copies.push( (base_address, buffer) );
            }
        },

        // If we are looking for a module, we only need to copy the memory associated with it
        CodeInjectionFileParsing::SearchType::module_name =>
        {
            // Get the right module from all of them
            let modules_r = process.get_modules();

            let modules: Vec<GenericOSInterface::ProcessModule> = match modules_r
            {
                Ok(value) => value,
                Err(error) => return Err(CodeInjectionErrors::OSInterfaceErrors(error))
            };

            // This should never panic if the parsing is correct. Besides, any panic should trigger the tests
            let target_module_r = modules.iter().position(|m| m.module_name == module_name.clone().unwrap());

            let target_module_pos: usize = match target_module_r
            {
                Some(value) => value,
                None => 
                {
                    eprintln!("Error module not found: \"{}\"", module_name.clone().unwrap());
                    return Err(CodeInjectionErrors::ModuleNotFound);
                }
            };

            let target_module: GenericOSInterface::ProcessModule = modules[target_module_pos].clone();

            // We got the right module, now copy its contents
            let mut buffer: Vec<u8> = vec![0; target_module.size];
            let os_result = process.read_from_vm(target_module.base_address, &mut buffer);

            match os_result
            {
                Ok(_) => {}, // Do nothing
                Err(error) => return Err(CodeInjectionErrors::OSInterfaceErrors(error))
            }

            region_copies.push( (target_module.base_address, buffer) );
        }
    }

    return Ok(region_copies);
}

// Inject code into the process and on success, return where each instruction was replaced
fn find_injection_code_address(memory_regions_payload: Vec< (usize, Vec<u8>) >,
                            instructions: Vec<CodeInjectionFileParsing::InjectionEntry>)

                            -> Result< Vec<(CodeInjectionFileParsing::InjectionEntry, Vec<usize>)>, CodeInjectionErrors>
{

    let mut instructions_abs_addresses: Vec<(CodeInjectionFileParsing::InjectionEntry, Vec<usize>)> = vec![];

    // Look for the instructions in the copied regions
    for instruc in instructions
    {
        let mut matches: usize = 0;
        let mut absolute_vm_addresses: Vec<usize> = vec![];
        
        for region_copy in &memory_regions_payload
        {
            // Just renaming it to better names
            let region_base_address = region_copy.0;
            let region_buffer = &region_copy.1;

            for payload_idx in 0..region_buffer.len()
            {
                // Match the instruction
                // This could be a new search engine (TODO)
                let mut did_it_match: bool = true;

                for byte_idx in 0..instruc.instruction.len()
                {
                    if payload_idx + byte_idx >= region_buffer.len() || instruc.instruction[byte_idx] != region_buffer[payload_idx + byte_idx]
                    {
                        did_it_match = false;
                        break;
                    }
                }

                if did_it_match == true
                {
                    matches += 1;
                    absolute_vm_addresses.push(payload_idx + region_base_address);
                }
            }
        }

        // Did we find it?
        // No
        if matches == 0
        {
            eprintln!("Could not find the instruction: {:?}", instruc);
            return Err(CodeInjectionErrors::InstructionNotFound);
        }

        if instruc.matches_allowed != None && matches > instruc.matches_allowed.unwrap()
        {
            eprintln!("Found more than the allowed matches: {}/{} \n Instruction: {:?}", matches, instruc.matches_allowed.unwrap(), instruc);
            return Err(CodeInjectionErrors::FoundMoreThanMatchesAllowed);
        }

        // Yes
        instructions_abs_addresses.push( (instruc, absolute_vm_addresses) );
    }

    // Everything went fine
    return Ok(instructions_abs_addresses);
}

#[allow(unused_assignments)]
fn inject_code(injection_addresses: &Vec<(CodeInjectionFileParsing::InjectionEntry, Vec<usize>)>, process: &mut GenericOSInterface::GenericProcess, dry_run: bool) -> Result<(), CodeInjectionErrors>
{
    for injection in injection_addresses
    {
        let instruction = injection.0.clone();
        let absolute_addresses = &injection.1;

        let mut range_start: usize = 0;
        let mut range_size: usize = 0;

        // If the range is empty, consider that we need to replace the whole input instruction
        if instruction.range == None
        {
            range_start = 0;
            range_size = instruction.instruction.len();
        }

        else
        {
            range_start = instruction.range.unwrap().0;
            range_size = instruction.range.unwrap().1;
        }

        for address in absolute_addresses
        {
            let nop_buffer: Vec<u8> = vec![nop_instruc; range_size];
            let injection_address = address + range_start;

            println!("Injecting code. Instruction: {:?} \nAddress: {}", nop_buffer, injection_address);

            if dry_run == false
            {
                let os_result = process.write_into_vm(&nop_buffer, injection_address);
                
                // Let the user know about errors, but don't abort
                if os_result.is_err()
                {
                    eprintln!("Error when writing code into the process!");
                }
            }
        }
    }

    return Ok(());
}

#[allow(unused_assignments)]
fn restore_code(injection_addresses: &Vec<(CodeInjectionFileParsing::InjectionEntry, Vec<usize>)>, process: &mut GenericOSInterface::GenericProcess, dry_run: bool) -> Result<(), CodeInjectionErrors>
{

    for injection in injection_addresses
    {
        let instruction = injection.0.clone();
        let absolute_addresses = &injection.1;

        let mut range_start: usize = 0;
        let mut range_size: usize = 0;

        // If the range is empty, consider that we need to replace the whole input instruction
        if instruction.range == None
        {
            range_start = 0;
            range_size = instruction.instruction.len();
        }

        else
        {
            range_start = instruction.range.unwrap().0;
            range_size = instruction.range.unwrap().1;
        }

        for address in absolute_addresses
        {
            let restore_buffer = &instruction.instruction[range_start..range_start+range_size];
            let injection_address = address + range_start;
 
            println!("Restoring code. Instruction: {:?} \nAddress: {}", restore_buffer, injection_address);

            if dry_run == false
            {
                let os_result = process.write_into_vm(restore_buffer, injection_address);
                
                // Let the user know about errors, but don't abort
                if os_result.is_err()
                {
                    eprintln!("Could not restore code. Error when writing code into the process!");
                }
            }

            println!("Successful restore");
        }
    }

    return Ok(());

}

// This fuction simply glues together the code injection helper functions
// In this way, I can test parts of the code injection independently
fn main_code_injection_flow(code_injection_config: CodeInjectionFileParsing::InjectionConfiguration, process: &mut GenericOSInterface::GenericProcess, dry_run: bool, wait_for_user: bool) -> Result<(), CodeInjectionErrors>
{
    // Copy all executable regions
    let copied_mem_regions_r = copy_memory_regions(code_injection_config.search_type.unwrap(), code_injection_config.module_name, process);

    let copied_mem_regions = match copied_mem_regions_r
    {
        Ok(value) => value,
        Err(error) => return Err(error)
    };

    // Search for the code addresses
    let injection_addresses_r = find_injection_code_address(copied_mem_regions, code_injection_config.instructions);

    let injection_addresses = match injection_addresses_r
    {
        Ok(value) => value,
        Err(error) => return Err(error)
    };

    // Inject the code
    let injection_r = inject_code(&injection_addresses, process, dry_run);

    match injection_r
    {
        Ok(_) => {},
        Err(error) => return Err(error)
    };

    // Wait for user input to restore the original code
    println!("Press ENTER to continue and restore the original instructions...");

    // This is mostly to help in uni testing
    if wait_for_user == true
    {
        let mut command = String::new();
        std::io::stdout().flush().unwrap();
        io::stdin().read_line(&mut command).expect("failed to read line");
    }

    // Restore the code
    let restoration_r = restore_code(&injection_addresses, process, dry_run);

    match restoration_r
    {
        Ok(_) => {},
        Err(error) => return Err(error)
    };

    return Ok(());
}

#[cfg(test)]
mod tests
{
    // Import the current module to all tests
    use crate::CodeInjection::*;
    use crate::CodeInjectionFileParsing::*;
    use crate::GenericOSInterface::*;

    #[test]
    fn TestCodeInjection_RegularCase()
    {

        let modules = vec![
                ProcessModule::new("module.exe".to_string(), 100, 100),
                ProcessModule::new("lib.dll".to_string(), 800, 100),
            ];

        let mut memory_regions = vec![
                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 100, 100),
                    vec![1; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100),
                    vec![2; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 800, 100),
                    vec![3; 100]),
            ];

        memory_regions[0].payload[50] = 0xAA;
        memory_regions[0].payload[51] = 0xAA;
        memory_regions[0].payload[52] = 0xAA;
        memory_regions[0].payload[53] = 0xAA;
    
        let mut process = GenericProcess::create(
            1,
            memory_regions.clone(),
            modules.clone(),
        );

        let needle = InjectionEntry::new(vec![0xAA, 0xAA, 0xAA, 0xAA], None, 1);
        let configuration = InjectionConfiguration::new(Some(SearchType::module_name), Some("module.exe".to_string()), vec![needle]);

        let injection_result = main_code_injection_flow(configuration, &mut process, false, false);

        assert_eq!(Ok(()), injection_result);

        // Did the process stay the same?
        assert_eq!(process.custom_image[0].payload, memory_regions[0].payload);
    }

    #[test]
    fn TestCodeInjection_CopyMemoryRegions_ModuleName()
    {

        let modules = vec![
                ProcessModule::new("module.exe".to_string(), 100, 100),
                ProcessModule::new("lib.dll".to_string(), 800, 100),
            ];

        let memory_regions = vec![
                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 100, 100),
                    vec![1; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100),
                    vec![2; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 800, 100),
                    vec![3; 100]),
            ];
    
        let process = GenericProcess::create(
            1,
            memory_regions.clone(),
            modules.clone(),
        );

        let region_copies = copy_memory_regions(SearchType::module_name, Some("module.exe".to_string()), &process).unwrap();

        // Did it copy only the module's memory?
        let expect: Vec<(usize, Vec<u8>)> = vec![(100, vec![1; 100])];
        assert_eq!(expect, region_copies);
    }

    #[test]
    fn TestCodeInjection_CopyMemoryRegions_ExecutableMemory()
    {

        let modules = vec![
                ProcessModule::new("module.exe".to_string(), 100, 100),
                ProcessModule::new("lib.dll".to_string(), 800, 100),
            ];

        let memory_regions = vec![
                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 100, 100),
                    vec![1; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100),
                    vec![2; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 800, 100),
                    vec![3; 100]),
            ];
    
        let process = GenericProcess::create(
            1,
            memory_regions.clone(),
            modules.clone(),
        );

        let region_copies = copy_memory_regions(SearchType::exe_memory, None, &process).unwrap();

        // Did it copy all of the executable regions?
        let expect: Vec<(usize, Vec<u8>)> = vec![
            (100, vec![1; 100]),
            (500, vec![2; 100]),
            (800, vec![3; 100])
            ];

        assert_eq!(expect, region_copies);
    }

    #[test]
    fn TestCodeInjection_CopyMemoryRegions_ModuleNotFound()
    {

        let modules = vec![
                ProcessModule::new("module.exe".to_string(), 100, 100),
                ProcessModule::new("lib.dll".to_string(), 800, 100),
            ];

        let memory_regions = vec![
                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 100, 100),
                    vec![1; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100),
                    vec![2; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 800, 100),
                    vec![3; 100]),
            ];
    
        let process = GenericProcess::create(
            1,
            memory_regions.clone(),
            modules.clone(),
        );

        let copy_result = copy_memory_regions(SearchType::module_name, Some("module_that_does_not_exist.exe".to_string()), &process);

        assert_eq!(Err(CodeInjectionErrors::ModuleNotFound), copy_result);
    }

    #[test]
    fn TestCodeInjection_FindInjectionCodeAdress_Start()
    {

        let modules = vec![
                ProcessModule::new("module.exe".to_string(), 100, 100),
                ProcessModule::new("lib.dll".to_string(), 800, 100),
            ];

        let mut memory_regions = vec![
                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 100, 100),
                    vec![1; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100),
                    vec![2; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 800, 100),
                    vec![3; 100]),
            ];

        memory_regions[0].payload[0] = 0xAA;
        memory_regions[0].payload[1] = 0xAA;
        memory_regions[0].payload[2] = 0xAA;
        memory_regions[0].payload[3] = 0xAA;
    
        let process = GenericProcess::create(
            1,
            memory_regions.clone(),
            modules.clone(),
        );

        let needle = InjectionEntry::new(vec![0xAA, 0xAA, 0xAA, 0xAA], None, 1);

        let region_copies = copy_memory_regions(SearchType::exe_memory, None, &process).unwrap();

        let address_to_inject = find_injection_code_address(region_copies, vec![needle.clone()]).unwrap();

        let expect: Vec<(InjectionEntry, Vec<usize>)> = vec![
            (needle.clone(), vec![100])
            ];

        assert_eq!(address_to_inject, expect);
    }

    #[test]
    fn TestCodeInjection_FindInjectionCodeAdress_Middle()
    {

        let modules = vec![
                ProcessModule::new("module.exe".to_string(), 100, 100),
                ProcessModule::new("lib.dll".to_string(), 800, 100),
            ];

        let mut memory_regions = vec![
                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 100, 100),
                    vec![1; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100),
                    vec![2; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 800, 100),
                    vec![3; 100]),
            ];

        memory_regions[0].payload[50] = 0xAA;
        memory_regions[0].payload[51] = 0xAA;
        memory_regions[0].payload[52] = 0xAA;
        memory_regions[0].payload[53] = 0xAA;
    
        let process = GenericProcess::create(
            1,
            memory_regions.clone(),
            modules.clone(),
        );

        let needle = InjectionEntry::new(vec![0xAA, 0xAA, 0xAA, 0xAA], None, 1);

        let region_copies = copy_memory_regions(SearchType::exe_memory, None, &process).unwrap();

        let address_to_inject = find_injection_code_address(region_copies, vec![needle.clone()]).unwrap();

        let expect: Vec<(InjectionEntry, Vec<usize>)> = vec![
            (needle.clone(), vec![150])
            ];

        assert_eq!(address_to_inject, expect);
    }

    #[test]
    fn TestCodeInjection_FindInjectionCodeAdress_End()
    {

        let modules = vec![
                ProcessModule::new("module.exe".to_string(), 100, 100),
                ProcessModule::new("lib.dll".to_string(), 800, 100),
            ];

        let mut memory_regions = vec![
                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 100, 100),
                    vec![1; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100),
                    vec![2; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 800, 100),
                    vec![3; 100]),
            ];

        memory_regions[0].payload[96] = 0xAA;
        memory_regions[0].payload[97] = 0xAA;
        memory_regions[0].payload[98] = 0xAA;
        memory_regions[0].payload[99] = 0xAA;
    
        let process = GenericProcess::create(
            1,
            memory_regions.clone(),
            modules.clone(),
        );

        let needle = InjectionEntry::new(vec![0xAA, 0xAA, 0xAA, 0xAA], None, 1);

        let region_copies = copy_memory_regions(SearchType::exe_memory, None, &process).unwrap();

        let address_to_inject = find_injection_code_address(region_copies, vec![needle.clone()]).unwrap();

        let expect: Vec<(InjectionEntry, Vec<usize>)> = vec![
            (needle.clone(), vec![196])
            ];

        assert_eq!(address_to_inject, expect);
    }

    #[test]
    fn TestCodeInjection_FindInjectionCodeAdress_MultipleRegions()
    {

        let modules = vec![
                ProcessModule::new("module.exe".to_string(), 100, 100),
                ProcessModule::new("lib.dll".to_string(), 800, 100),
            ];

        let mut memory_regions = vec![
                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 100, 100),
                    vec![1; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100),
                    vec![2; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 800, 100),
                    vec![3; 100]),
            ];

        memory_regions[0].payload[0] = 0xAA;
        memory_regions[0].payload[1] = 0xAA;
        memory_regions[0].payload[2] = 0xAA;
        memory_regions[0].payload[3] = 0xAA;

        memory_regions[1].payload[50] = 0xAA;
        memory_regions[1].payload[51] = 0xAA;
        memory_regions[1].payload[52] = 0xAA;
        memory_regions[1].payload[53] = 0xAA;

        memory_regions[2].payload[96] = 0xAA;
        memory_regions[2].payload[97] = 0xAA;
        memory_regions[2].payload[98] = 0xAA;
        memory_regions[2].payload[99] = 0xAA;
    
        let process = GenericProcess::create(
            1,
            memory_regions.clone(),
            modules.clone(),
        );

        let needle = InjectionEntry::new(vec![0xAA, 0xAA, 0xAA, 0xAA], None, 3);

        let region_copies = copy_memory_regions(SearchType::exe_memory, None, &process).unwrap();

        let address_to_inject = find_injection_code_address(region_copies, vec![needle.clone()]).unwrap();

        let expect: Vec<(InjectionEntry, Vec<usize>)> = vec![
            (needle.clone(), vec![100, 550, 896])
            ];

        assert_eq!(address_to_inject, expect);
    }

    #[test]
    fn TestCodeInjection_FindInjectionCodeAdress_Error_InstructionNotFound()
    {

        let modules = vec![
                ProcessModule::new("module.exe".to_string(), 100, 100),
                ProcessModule::new("lib.dll".to_string(), 800, 100),
            ];

        let memory_regions = vec![
                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 100, 100),
                    vec![1; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100),
                    vec![2; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 800, 100),
                    vec![3; 100]),
            ];
    
        let process = GenericProcess::create(
            1,
            memory_regions.clone(),
            modules.clone(),
        );

        let needle = InjectionEntry::new(vec![0xAA, 0xAA, 0xAA, 0xAA], None, 3);

        let region_copies = copy_memory_regions(SearchType::exe_memory, None, &process).unwrap();

        let address_to_inject_r = find_injection_code_address(region_copies, vec![needle.clone()]);

        assert_eq!(address_to_inject_r, Err(CodeInjectionErrors::InstructionNotFound));
    }

    #[test]
    fn TestCodeInjection_FindInjectionCodeAdress_TooManyMatches()
    {

        let modules = vec![
                ProcessModule::new("module.exe".to_string(), 100, 100),
                ProcessModule::new("lib.dll".to_string(), 800, 100),
            ];

        let mut memory_regions = vec![
                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 100, 100),
                    vec![1; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100),
                    vec![2; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 800, 100),
                    vec![3; 100]),
            ];

        memory_regions[0].payload[0] = 0xAA;
        memory_regions[0].payload[1] = 0xAA;
        memory_regions[0].payload[2] = 0xAA;
        memory_regions[0].payload[3] = 0xAA;

        memory_regions[1].payload[50] = 0xAA;
        memory_regions[1].payload[51] = 0xAA;
        memory_regions[1].payload[52] = 0xAA;
        memory_regions[1].payload[53] = 0xAA;
    
        let process = GenericProcess::create(
            1,
            memory_regions.clone(),
            modules.clone(),
        );

        let needle = InjectionEntry::new(vec![0xAA, 0xAA, 0xAA, 0xAA], None, 1);

        let region_copies = copy_memory_regions(SearchType::exe_memory, None, &process).unwrap();

        let address_to_inject_r = find_injection_code_address(region_copies, vec![needle.clone()]);

        assert_eq!(address_to_inject_r, Err(CodeInjectionErrors::FoundMoreThanMatchesAllowed));
    }

    #[test]
    fn TestCodeInjection_CodeInjection()
    {

        let modules = vec![
                ProcessModule::new("module.exe".to_string(), 100, 100),
                ProcessModule::new("lib.dll".to_string(), 800, 100),
            ];

        let mut memory_regions = vec![
                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 100, 100),
                    vec![1; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100),
                    vec![2; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 800, 100),
                    vec![3; 100]),
            ];

        memory_regions[0].payload[0] = 0xAA;
        memory_regions[0].payload[1] = 0xAA;
        memory_regions[0].payload[2] = 0xAA;
        memory_regions[0].payload[3] = 0xAA;
    
        let mut process = GenericProcess::create(
            1,
            memory_regions.clone(),
            modules.clone(),
        );

        let needle = InjectionEntry::new(vec![0xAA, 0xAA, 0xAA, 0xAA], None, 3);

        let region_copies = copy_memory_regions(SearchType::exe_memory, None, &process).unwrap();

        let address_to_inject = find_injection_code_address(region_copies, vec![needle.clone()]).unwrap();

        let _ = inject_code(&address_to_inject, &mut process, false).unwrap();

        let mut expect: Vec<u8> = memory_regions[0].payload.clone();
        expect[0] = 0x90;
        expect[1] = 0x90;
        expect[2] = 0x90;
        expect[3] = 0x90;

        assert_eq!(process.custom_image[0].payload, expect);
    }

    #[test]
    fn TestCodeInjection_CodeInjection_RangeStart()
    {

        let modules = vec![
                ProcessModule::new("module.exe".to_string(), 100, 100),
                ProcessModule::new("lib.dll".to_string(), 800, 100),
            ];

        let mut memory_regions = vec![
                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 100, 100),
                    vec![1; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100),
                    vec![2; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 800, 100),
                    vec![3; 100]),
            ];

        memory_regions[0].payload[0] = 0x01;
        memory_regions[0].payload[1] = 0x02;
        memory_regions[0].payload[2] = 0x03; // I want to replace from here
        memory_regions[0].payload[3] = 0x04;
        memory_regions[0].payload[4] = 0x05;
        memory_regions[0].payload[5] = 0x06;
        memory_regions[0].payload[6] = 0x07;
    
        let mut process = GenericProcess::create(
            1,
            memory_regions.clone(),
            modules.clone(),
        );

        let needle = InjectionEntry::new(vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07], Some((2, 5)), 1);

        let region_copies = copy_memory_regions(SearchType::exe_memory, None, &process).unwrap();

        let address_to_inject = find_injection_code_address(region_copies, vec![needle.clone()]).unwrap();

        let _ = inject_code(&address_to_inject, &mut process, false).unwrap();

        let mut expect: Vec<u8> = memory_regions[0].payload.clone();
        expect[2] = 0x90;
        expect[3] = 0x90;
        expect[4] = 0x90;
        expect[5] = 0x90;
        expect[6] = 0x90;

        assert_eq!(process.custom_image[0].payload, expect);
    }

    #[test]
    fn TestCodeInjection_CodeInjection_RangeSize()
    {

        let modules = vec![
                ProcessModule::new("module.exe".to_string(), 100, 100),
                ProcessModule::new("lib.dll".to_string(), 800, 100),
            ];

        let mut memory_regions = vec![
                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 100, 100),
                    vec![1; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100),
                    vec![2; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 800, 100),
                    vec![3; 100]),
            ];

        memory_regions[0].payload[0] = 0x01;
        memory_regions[0].payload[1] = 0x02;
        memory_regions[0].payload[2] = 0x03; // I want to replace from here
        memory_regions[0].payload[3] = 0x04;
        memory_regions[0].payload[4] = 0x05;
        memory_regions[0].payload[5] = 0x06; // And stop here
        memory_regions[0].payload[6] = 0x07;
    
        let mut process = GenericProcess::create(
            1,
            memory_regions.clone(),
            modules.clone(),
        );

        let needle = InjectionEntry::new(vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07], Some((2, 4)), 1);

        let region_copies = copy_memory_regions(SearchType::exe_memory, None, &process).unwrap();

        let address_to_inject = find_injection_code_address(region_copies, vec![needle.clone()]).unwrap();

        let _ = inject_code(&address_to_inject, &mut process, false).unwrap();

        let mut expect: Vec<u8> = memory_regions[0].payload.clone();
        expect[2] = 0x90;
        expect[3] = 0x90;
        expect[4] = 0x90;
        expect[5] = 0x90;

        assert_eq!(process.custom_image[0].payload, expect);
    }

    #[test]
    fn TestCodeInjection_CodeRestoration()
    {

        let modules = vec![
                ProcessModule::new("module.exe".to_string(), 100, 100),
                ProcessModule::new("lib.dll".to_string(), 800, 100),
            ];

        let mut memory_regions = vec![
                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 100, 100),
                    vec![1; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100),
                    vec![2; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 800, 100),
                    vec![3; 100]),
            ];

        memory_regions[0].payload[0] = 0xAA;
        memory_regions[0].payload[1] = 0xAA;
        memory_regions[0].payload[2] = 0xAA;
        memory_regions[0].payload[3] = 0xAA;
        memory_regions[0].payload[4] = 0xAA; // Don't replace this one 

        memory_regions[0].payload[20] = 0xAA;
        memory_regions[0].payload[21] = 0xAA;
        memory_regions[0].payload[22] = 0xAA;
        memory_regions[0].payload[23] = 0xAA;
        memory_regions[0].payload[24] = 0xAA; // Don't replace this one 

        memory_regions[0].payload[50] = 0xBB;
        memory_regions[0].payload[51] = 0xBB;
        memory_regions[0].payload[52] = 0xBB;
        memory_regions[0].payload[53] = 0xBB;

        memory_regions[1].payload[50] = 0xCC;
        memory_regions[1].payload[51] = 0xCC; // Replace from here
        memory_regions[1].payload[52] = 0xCC;
        memory_regions[1].payload[53] = 0xCC; // Stop here
        memory_regions[1].payload[54] = 0xCC;
        memory_regions[1].payload[55] = 0xCC;
    
        let mut process = GenericProcess::create(
            1,
            memory_regions.clone(),
            modules.clone(),
        );

        let needles = vec![
            InjectionEntry::new(vec![0xAA, 0xAA, 0xAA, 0xAA, 0xAA], Some((0, 4)), 2),
            InjectionEntry::new(vec![0xBB, 0xBB, 0xBB, 0xBB], None, 1),
            InjectionEntry::new(vec![0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC], Some((1, 3)), 1),
        ];

        let configuration = InjectionConfiguration::new(Some(SearchType::exe_memory), None, needles);

        let injection_result = main_code_injection_flow(configuration, &mut process, false, false);

        assert_eq!(Ok(()), injection_result);

        // Did the process stay the same?
        assert_eq!(process.custom_image[0].payload, memory_regions[0].payload);
        assert_eq!(process.custom_image[1].payload, memory_regions[1].payload);
    }
}
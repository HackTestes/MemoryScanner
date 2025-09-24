
#[derive(Debug)]
#[derive(PartialEq)]
pub enum TokenizationError
{
    UnclosedQuotes,
    EmptyString
}

fn is_empty_space(character: char) -> bool
{
    return character == ' ' || character == '\n' || character == '\t'
}

pub fn tokenize(string_text: String) -> Result<Vec<String>, TokenizationError>
{
    let mut tokens: Vec<String> = vec![];

    let mut token: String = String::new();

    // Keep track of quotes
    let mut open_single_quote: bool = false;
    let mut open_double_quotes: bool = false;

    for character in string_text.chars()
    {
        /*
        println!("CHARACTER> {}", character);
        println!("TOKEN> {}", token);
        println!("TOKENS> {:?}", tokens);
        println!("OPEN SINGLE QUOTE> {}", open_single_quote);
        println!("OPEN SINGLE QUOTE CMP> {}", character == '\'');
        println!("OPEN DOUBLE QUOTES> {}", open_double_quotes);
        println!("OPEN DOUBLE QUOTES CMP> {}", character == '\"');
        println!("\n\n");
        */

        // If it is a space
        if is_empty_space(character) && open_single_quote == false && open_double_quotes == false
        {
            // If there is a token being written, this must be the end of one
            if token.len() != 0
            {
                // Push it and start over
                tokens.push(token);
                token = String::new();
            }

            continue;
        }

        if character == '\'' && open_double_quotes == false
        {
            // Opening a quote statement
            if open_single_quote == false
            {
                open_single_quote = true;
            }

            // Closing one
            else
            {
                open_single_quote = false;
            }

            continue; 
        }

        if character == '\"' && open_single_quote == false
        {
            // Opening a quote statement
            if open_double_quotes == false
            {
                open_double_quotes = true;
            }

            // Closing one
            else
            {
                open_double_quotes = false;
            }

            continue; 
        }

        // If is a token start (aka not a space)
        else
        {
            token.push(character);
            continue;
        }
    }

    // If we finished the process with a token yet to pushed (end quote is the last character)
    if token.len() != 0
    {
        tokens.push(token);
    }

    // Did you forget to close the quotes?
    if open_single_quote == true || open_double_quotes == true
    {
        return Err(TokenizationError::UnclosedQuotes);
    }

    // Is it an empty String?
    if tokens.len() == 0
    {
        return Err(TokenizationError::EmptyString);
    }

    return Ok(tokens);
}


#[cfg(test)]
mod tests
{
    use crate::Tokenization::*;

    // Action tests
   #[test]
    fn TokenTest_RegularCase()
    {
        assert_eq!(
            tokenize("what I want is to become \"a new token!\"".to_string()).unwrap(),
            vec!["what".to_string(), "I".to_string(), "want".to_string(), "is".to_string(), "to".to_string(), "become".to_string(), "a new token!".to_string()]);
    }

   #[test]
    fn TokenTest_Spaces()
    {
        assert_eq!(
            tokenize("Spaces      \"don't\" matter".to_string()).unwrap(),
            vec!["Spaces".to_string(), "don't".to_string(), "matter".to_string()]);
    }


   #[test]
    fn TokenTest_Path()
    {
        assert_eq!(
            tokenize("I want merged paths: C:\\path\\\"a dir with spaces\"\\executable.exe".to_string()).unwrap(),
            vec!["I".to_string(), "want".to_string(), "merged".to_string(), "paths:".to_string(), "C:\\path\\a dir with spaces\\executable.exe".to_string()]);
    }

    #[test]
    fn TokenTest_UnclosedQuotes()
    {
        assert_eq!(
            tokenize("what I want to become: \"a new token!".to_string()),
            Err(TokenizationError::UnclosedQuotes));
    }

    #[test]
    fn TokenTest_UnclosedQuotes_Mixed()
    {
        assert_eq!(
            tokenize("what I want to become: \"a new token!\". So Now \'unclose it".to_string()),
            Err(TokenizationError::UnclosedQuotes));
    }

    #[test]
    fn TokenTest_EmptyString()
    {
        assert_eq!(
            tokenize("".to_string()),
            Err(TokenizationError::EmptyString));
    }

}
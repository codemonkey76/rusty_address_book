use std::fmt::{Display, Formatter};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{stdout, Read, Write, self, BufRead, BufReader, ErrorKind, Stdout};
use std::io::ErrorKind::InvalidInput;
use std::path::{PathBuf, Path};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crossterm:: {
    execute,
    event::{poll, read, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
    cursor::MoveTo,
};
use ctrlc;
extern crate directories;
use directories::{BaseDirs, UserDirs, ProjectDirs};
use regex::Regex;


struct RawMode;

impl RawMode {
    fn enable() -> io::Result<RawMode> {
        enable_raw_mode()?;
        Ok(RawMode)
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

#[derive(Parser, Debug)]
#[clap(author="Shane Poppleton", version, about="Very simple command line phone directory")]
struct Args {
    #[arg(short, long)]
    filename: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
enum Identifier {
    Name(String),
    Company(String),
    Both(String, String),
}

#[derive(Serialize, Deserialize, Debug)]
struct Customer {
    identifier: Identifier,
    phone: String,
}

impl Display for Customer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.identifier {
            Identifier::Name(name) => write!(f, "  Name: {}, ", name),
            Identifier::Company(company) => write!(f, "  Company: {}, ", company),
            Identifier::Both(name, company) => write!(f, "  Company: {}, Name: {}, ", company, name),
        }?;
        writeln!(f, "Phone: {}\r", self.phone)
    }
}

enum InputCommand {
    Add { params: Vec<String> },
    Delete { params: Vec<String> },
    Search { params: Vec<String> },
    List,
    Help,
    Quit
}

fn init() -> crossterm::Result<(Arc<RawMode>, Arc<AtomicBool>)> {
    let raw_mode = Arc::new(RawMode::enable()?);
    let running = Arc::new(AtomicBool::new(true));
    let r = raw_mode.clone();
    let r2 = running.clone();

    ctrlc::set_handler(move || {
        r2.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    Ok((raw_mode, running))
}
fn sample_data() -> Vec<Customer> {
    vec![
        Customer {
            identifier: Identifier::Name("Alice".to_string()),
            phone: "1234567890".to_string(),
        },
        Customer {
            identifier: Identifier::Company("Acme Inc".to_string()),
            phone: "9876543210".to_string(),
        },
        Customer {
            identifier: Identifier::Both("Test Company".to_string(), "Bob".to_string()),
            phone: "10293848576".to_string(),
        }
    ]
}
fn query_prompt(mut stdout: &Stdout) {
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0)).expect("Could not clear screen");
    print!("Query: ");
    execute!(stdout, MoveTo(0,2));
}

fn main() -> crossterm::Result<()> {
    let (raw_mode, running) = init()?;

    while running.load(Ordering::SeqCst) {
        let customers = sample_data();
        let mut query = String::new();
        let mut filtered = filter_customers(&customers, &query);
        let mut stdout = stdout();
        query_prompt(&stdout);

        display_customers(&filtered);
        execute!(stdout, MoveTo(7, 0))?;
        loop {
            if poll(std::time::Duration::from_millis(500))? {
                if let Event::Key(event) = read()? {
                    match event.code {
                        KeyCode::Char('c') if event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                            running.store(false, Ordering::SeqCst);
                            break;
                        }
                        KeyCode::Char(c) => {
                            query.push(c);
                        }
                        KeyCode::Backspace if !query.is_empty() => {
                            query.pop();
                        }
                        KeyCode::Enter => {
                            break;
                        }
                        _ => {}
                    }
                    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;

                    filtered = filter_customers(&customers, &query);
                    display_customers(&filtered);
                }
            }
        }

    }

    Ok(())

    /*
    let args = Args::parse();

    let mut fname: String;

    if let Some(filename) = get_customer_file(args.filename) {
        println!("Using customer file: {}", &filename);
        fname = filename;
    } else {
        eprintln!("Error: Could not find customer file");
        std::process::exit(1);
    }

    let mut customers: Vec<Customer> = Vec::new();

    if let Err(e) = load_customers(&fname, &mut customers) {
        eprintln!("Error loading customers: {}", e)
    }

    print_welcome();

    loop {
        prompt();
        match process_input() {
            Ok(InputCommand::Add{params}) => add_customer(params),
            Ok(InputCommand::Delete { params}) => delete_customer(params),
            Ok(InputCommand::Search{params}) => find_customers(params, &customers),
            Ok(InputCommand::List) => list_customers(&customers),
            Ok(InputCommand::Help) => print_help(),
            Ok(InputCommand::Quit) => break,
            Err(e) => {
                eprintln!("Error processing input: {}", e);
            }
        }
    }


    if let Err(e) = save_customers(&fname, &customers) {
        eprintln!("Error saving customers: {}", e);
    }*/
}

fn display_customers(customers: &Vec<&Customer>) {
    for customer in customers {
        println!("{}", customer);
    }
}

fn filter_customers<'a>(customers: &'a Vec<Customer>, query: &str) -> Vec<&'a Customer> {
    let query = query.to_lowercase();
    customers
        .iter()
        .filter(|c|
            match &c.identifier {
                Identifier::Name(name) => name.to_lowercase().contains(&query),
                Identifier::Company(company) => company.to_lowercase().contains(&query),
                Identifier::Both(company, name) => name.to_lowercase().contains(&query) || company.to_lowercase().contains(&query),
            } || c.phone.to_lowercase().contains(&query)
        ).collect()
}

fn get_config_path() -> Option<String> {
    if let Some(proj_dirs) = ProjectDirs::from("au", "popplestones",  "Rusty Address Book") {
        let mut config_path: PathBuf = proj_dirs.config_dir().into();
        config_path.push("customers.json");
        config_path.to_str().map(String::from)
    } else {
        None
    }
}

fn get_customer_file(filename: Option<String>) -> Option<String> {
    match filename {
        Some(filename) => Some(filename),
        None => get_config_path()
    }
}

fn save_customers(filename: &str, customers: &Vec<Customer>) -> Result<(), Box<dyn std::error::Error>>{
    let json = serde_json::to_string(&customers)?;

    let mut file = OpenOptions::new().write(true).create(true).open(filename)?;

    Ok(file.write_all(json.as_bytes())?)
}

fn load_customers(filename: &str, customers: &mut Vec<Customer>) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(filename);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(filename)?;

    let reader = BufReader::new(file);
    let loaded_customers: Result<Vec<Customer>, _> = serde_json::from_reader(reader);

    match loaded_customers {
        Ok(mut data) => {
            customers.clear();
            customers.append(&mut data);
            Ok(())
        },
        Err(err) if err.is_eof() => {
            customers.clear();
            Ok(())
        },
        Err(err) => Err(Box::new(err)),
    }
}

fn list_customers(customers: &Vec<Customer>) {
    println!("All Customers:");
    for customer in customers {
        println!("{}", customer);
    }
}

/*
fn add_customer(params: Vec<String>, customers: &mut Vec<Customer>) {
    if params.len() == 2 {
        name = params.get(0).ok_or("No param")?;
        phone = params.get(1).ok_or("No param")?;
    }

    if params.len() == 1 {
        name = params.get(0).ok_or("No param")?;
    }
    println!("{:?}", params);
}
*/

fn delete_customer(params: Vec<String>) {

}

/*
fn find_customers(params: Vec<String>, customers: &Vec<Customer>){
    let search = params.first().unwrap_or(&"".to_string()).to_lowercase();

    let filtered: Vec<&Customer> =
        customers
            .iter()
            .filter(|customer| customer.name.to_lowercase().contains(&search))
            .collect();

    println!("Search Results:");

    if filtered.len() == 0 {
        println!("    (none)");
    }

    for customer in filtered {
        println!("{}", customer);
    }
}
*/

fn print_help() {
    println!("USAGE:");
    println!("/list                 List all customers");
    println!("/add \"name\" \"phone\"   Add a customer with \"name\" as name and \"phone\" as phone");
    println!("/delete \"foo\"         Delete all customers matching \"foo\"");
    println!("/help                 This help screen");
    println!("/quit                 Quit the program");
    println!("foo                   Search for all customers matching \"foo\"");
}

fn print_welcome() {
    println!("***********************************************");
    println!("*        Welcome to Rusty Address Book        *");
    println!("*                                             *");
    println!("* Loading customers from : customers.json     *");
    println!("* Total Customers: 5                          *");
    println!("*                                             *");
    println!("* Type \"/help\" for help                      *");
    println!("***********************************************");
}

fn prompt() {
    print!("> ");
    io::stdout().flush().unwrap();
}

fn process_input() -> Result<InputCommand, Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut line = String::new();

    stdin.lock().read_line(&mut line)?;
    let line: Vec<_> = line.trim().splitn(2, ' ').collect();
    let command = line
        .get(0).ok_or("Invalid input")?
        .to_lowercase();

    let re = Regex::new(r#"([^"]*)"|'([^']*)'|(\S+)"#)?;
    let rest_of_command = line.get(1).ok_or("Invalid Input")?;

    let args =
        re.captures_iter(line.get(1).ok_or("Invalid input")?)
            .filter_map(|cap| cap.get(1).or(cap.get(2)).or(cap.get(3)))
            .map(|m| m.as_str().trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

    if command.starts_with('/') {
        let command = match command.as_str() {
            "/add" => InputCommand::Add { params: args },
            "/delete" => InputCommand::Delete { params: args },
            "/list" => InputCommand::List,
            "/help" => InputCommand::Help,
            "/quit" => InputCommand::Quit,
            _ => {
                println!("Unknown command: {}", command);
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Unknown command")));
            }
        };

        Ok(command)
    } else {
        Ok(InputCommand::Quit)
        // let mut search_options = vec![first_word.to_string()];
        // search_options.extend(options);
        //
        // Ok(InputCommand::Search { params: search_options })
    }
}
use std::fmt::{Display, Formatter};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write, self, BufRead};
extern crate directories;
use directories::{BaseDirs, UserDirs, ProjectDirs};

#[derive(Parser, Debug)]
#[clap(author="Shane Poppleton", version, about="Very simple command line phone directory")]
struct Args {
    #[arg(short, long)]
    filename: String,
}


#[derive(Serialize, Deserialize, Debug)]
struct Customer {
    name: String,
    phone: String,
}

impl Display for Customer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "    {} ", self.name)?;
        writeln!(f, "Phone: {}", self.phone)
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


fn main() {

    if let Some(proj_dirs) = ProjectDirs::from("com", "Foo Corp",  "Bar App") {
        proj_dirs.config_dir();
// Lin: /home/alice/.config/barapp
// Win: C:\Users\Alice\AppData\Roaming\Foo Corp\Bar App\config
// Mac: /Users/Alice/Library/Application Support/com.Foo-Corp.Bar-App
    }

    let args = Args::parse();
    let mut customers: Vec<Customer> = Vec::new();

    if let Err(e) = load_customers(&args.filename, &mut customers) {
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


    if let Err(e) = save_customers(&args.filename, &customers) {
        eprintln!("Error saving customers: {}", e);
    }
}

fn save_customers(filename: &str, customers: &Vec<Customer>) -> Result<(), Box<dyn std::error::Error>>{
    let json = serde_json::to_string(&customers)?;

    let mut file = OpenOptions::new().write(true).create(true).open(filename)?;

    Ok(file.write_all(json.as_bytes())?)
}

fn load_customers(filename: &str, customers: &mut Vec<Customer>) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::open(filename)?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    *customers = serde_json::from_str(&*contents)?;

    Ok(())
}

fn list_customers(customers: &Vec<Customer>) {
    println!("All Customers:");
    for customer in customers {
        println!("{}", customer);
    }
}

fn add_customer(params: Vec<String>) {

}

fn delete_customer(params: Vec<String>) {

}

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
    let line = line.trim().to_lowercase();
    let mut words = line.split_whitespace();

    if let Some(first_word) = words.next() {
        let command = first_word;
        let options: Vec<String> = words.map(|s| s.to_string()).collect();

        if command.starts_with('/') {
            let command = match command {
                "/add" => InputCommand::Add { params: options },
                "/delete" => InputCommand::Delete { params: options },
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
            let mut search_options = vec![first_word.to_string()];
            search_options.extend(options);

            Ok(InputCommand::Search { params: search_options })
        }
    } else {
        Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "No Input provided")))
    }

}
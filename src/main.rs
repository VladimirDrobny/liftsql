use std::io;
use std::io::{Read, Write};
use postgres::{Error, Transaction};

use std::fs::File;

use std::env;

use liftsql::Db;

use chrono::{NaiveDate, Datelike, Duration};

fn main() {
    let mut tui = Tui::new();
    tui.run();
}

struct Tui<'a> {
    db: Db,
    plan: Vec<(&'a str, Vec<(i32, Weight, Reps, i32)>)>,
}

struct Config {
    day_id: i32,
}

enum Weight {
    RMPer(f32),
    Static(f32),
}

enum Reps {
    AMRAP,
    Static(f32),
}

impl Config {
    fn default() -> Config {
        Config {day_id: 0,}
    }
}

impl Tui<'_> {
    fn new() -> Tui<'static> {
        let db = match Db::new("localhost", "postgres", &None, "liftsql") {
            Ok(db_) => db_,
            Err(err) => panic!("{}", err),
        };
        let plan = vec![("Volume Bench", vec![(1, Weight::RMPer(90.0), Reps::Static(5.0), 5), (2, Weight::RMPer(90.0), Reps::Static(5.0), 5), (3, Weight::RMPer(100.0), Reps::Static(5.0), 1)]), ("Recovery Press", vec![(1, Weight::RMPer(72.0), Reps::Static(5.0), 2), (4, Weight::RMPer(81.0), Reps::Static(5.0), 3), (5, Weight::Static(0.0), Reps::AMRAP, 1)]), ("PR Press", vec![(1, Weight::RMPer(100.0), Reps::Static(5.0), 1), (4, Weight::RMPer(100.0), Reps::Static(5.0), 1), (6, Weight::RMPer(100.0), Reps::Static(3.0), 5)]), ("Volume Press", vec![(1, Weight::RMPer(90.0), Reps::Static(5.0), 5), (4, Weight::RMPer(90.0), Reps::Static(5.0), 5), (3, Weight::RMPer(100.0), Reps::Static(5.0), 1)]), ("Recovery Bench", vec![(1, Weight::RMPer(72.0), Reps::Static(5.0), 2), (2, Weight::RMPer(81.0), Reps::Static(5.0), 3), (5, Weight::Static(0.0), Reps::AMRAP, 1)]), ("PR Bench", vec![(1, Weight::RMPer(100.0), Reps::Static(5.0), 1), (2, Weight::RMPer(100.0), Reps::Static(5.0), 1), (6, Weight::RMPer(100.0), Reps::Static(3.0), 5)])];
        Tui {db, plan}
    }

    const CONFIG_DIR: &str = ".config/.liftsql";

    fn read_config(&self) -> Result<Config, io::Error> {
        let mut config_path = match env::home_dir() {
            Some(path) => path,
            None => panic!("CAN'T READ HOME DIRECTORY."),
        };
        config_path.push(Tui::CONFIG_DIR);

        let mut file = File::open(config_path.as_path())?;
        let mut ret = String::new();
        file.read_to_string(&mut ret)?;
        Ok(Config {day_id: ret.parse().unwrap(),})
    }

    fn write_config(&self, config: &Config) -> Result<(), io::Error> {
        let mut config_path = match env::home_dir() {
            Some(path) => path,
            None => panic!("CAN'T READ HOME DIRECTORY."),
        };
        config_path.push(Tui::CONFIG_DIR);

        let mut file = File::create(config_path.as_path())?;
        //file.write_all(data.as_bytes())?;
        file.write_all(config.day_id.to_string().as_bytes())?;
        Ok(())
    }

    fn run(&mut self) {
        self.dialogue_menu();
    }

    fn get_user_input(prompt: &str) -> String {
        print!("{}",prompt);
        io::stdout().flush().expect("failed to flush buffer.");
        let mut buffer: String = String::new();
        io::stdin().read_line(&mut buffer).expect("Failed to read line.");
        String::from(buffer.trim())
    }

    fn get_user_input_float(prompt: &str, default: Option<f32>) -> Option<f32> {
        loop {
            let inp = Tui::get_user_input(&prompt);
            if inp == "q" || inp == "c" {
                return None;
            }
            if inp == "" {
                if let Some(def) = default {
                    return Some(def);
                }
            }
            match inp.parse::<f32>() {
                Ok(i) => return Some(i),
                Err(_) => {println!("Invalid input."); continue;},
            }
        }
    }

    fn dialogue_menu(&mut self) {
        if let Err(_) = self.print_last_session_ago() {
            println!("COULDN'T GET LAST SESSION INFO");
        }

        println!("n) New session\np) Show plan\ng) Get pr\na) Add exercise\nq) Quit");
        loop {
            println!("=====");
            let inp = Tui::get_user_input("$ ");
            match inp.as_str() {
                "n" => {
                    match self.dialogue_new_session() {
                        Ok(done) => {
                            match done {
                                true => println!("+ Session logged."),
                                false => println!("+ Session creation cancelled."),
                            };
                        },
                        Err(err) => println!("+ ERROR CREATING NEW SESSION: {}", err),
                    }
                },
                "p" => {
                    self.dialogue_plan();
                },
                "g" => {
                    match self.dialogue_get_pr() {
                        Ok(success) => {
                            if !success {
                                println!("Getting pr cancelled.");
                            }
                        },
                        Err(_) => println!("ERROR GETTING PR"),
                    };
                },
                "a" => {
                    match self.dialogue_add_exercise() {
                        Ok(success) => {
                            match success {
                                true => println!("Exercise added."),
                                false => println!("Exercise add cancelled."),
                            }
                        }
                        Err(_) => println!("ERROR ADDING EXERCISE"),
                    };
                },
                "q" => return,
                "c" => return,
                &_ => {
                    println!("Invalid input.");
                },
            }
        }
    }

    fn dialogue_plan(&mut self) {
        let mut config = match self.read_config() {
            Ok(conf) => conf,
            Err(err) => {println!("Error loading config: {}\nLoading default instead.", err); Config::default()}
        };
        loop {
            if let Err(err) = self.print_day(config.day_id) {
                println!("Error printing day: {}", err);
            }
            println!("-----");
            let inp = Tui::get_user_input("Plan (Next/Prev)# ");
            match inp.as_str() {
                "q" => break,
                "c" => break,
                "n" => {println!("Showing Next:");config.day_id = (config.day_id+1)%(self.plan.len() as i32);},
                "p" => {println!("Showing Prev:");config.day_id = (config.day_id-1)%(self.plan.len() as i32);},
                _ => println!("Invalid input."),
            };

            if let Err(err) = self.write_config(&config) {
                println!("ERROR SAVING CONFIG: {}", err);
            }
        }
    }

    fn print_day(&mut self, day_id: i32) -> Result<bool, Error> {
        let day = match self.plan.get(day_id as usize) {
            Some(d) => d,
            None => {println!("ERROR GETTING plan day id."); return Ok(false);}
        };
        println!("{}", day.0);
        for exercise in &day.1 {
            let name = self.db.select_exercise_name(exercise.0)?; 
            let weight = match exercise.1 {
                Weight::RMPer(percent) => {
                    match exercise.2 {
                        Reps::AMRAP => {println!("ERROR CALCULATING AMRAP PR WEIGHT. TODO."); return Ok(false);},
                        Reps::Static(r) => {
                            match self.db.select_exercise_weight_pr(exercise.0, r) {
                                Ok(w) => Some(w*percent/100.0),
                                Err(_) => None,
                            }
                        },
                    }
                },
                Weight::Static(w) => Some(w),
            };
            let reps = match exercise.2 {
                Reps::AMRAP => {
                    match weight {
                        Some(w) => {
                            match self.db.select_exercise_reps_pr(exercise.0, w) {
                                Ok(r) => Some(r),
                                Err(_) => None,
                            }
                        },
                        None => None,
                    }
                },
                Reps::Static(r) => Some(r),
            };

            let str_reps = match reps {
                Some(r) => r.to_string(),
                None => String::from("?"),
            };
            let str_weight = match weight {
                Some(w) => w.to_string(),
                None => String::from("?"),
            };

            let mut pr_reps = String::new();
            let mut pr_weight = String::new();
            let mut pr_sign = String::new();
            if let Reps::AMRAP = exercise.2 {
                pr_reps = String::from(">");
                pr_sign.push_str(" *AMRAP*");
            }
            if let Weight::RMPer(w) = exercise.1 {
                if w == 100.0 {
                    pr_weight = String::from(">");
                    pr_sign.push_str(" *PR*");
                }
            }
            println!("{} {}x{}{} {}{}kg{}", name, exercise.3, pr_reps, str_reps, pr_weight, str_weight, pr_sign);
        }
        Ok(true)
    }



    fn print_last_session_ago(&mut self) -> Result<(), Error> {
        match self.db.select_last_session_id() {
            Ok(last_session_id) => {
                let last_session_date = self.db.select_session_date(last_session_id)?;
                let current_date = self.db.select_current_date()?;
                let ago_str = Tui::get_ago_str(&last_session_date, &current_date);
                println!("Last session on {} {} ({})", last_session_date.weekday(), last_session_date.format("%d.%m."), ago_str);
            },
            Err(_) => println!("[No previous sessions]"),
        }
        Ok(())
    }

    fn get_ago_str(date_then: &NaiveDate, date_now: &NaiveDate) -> String {
        let ago_num = (*date_now - *date_then).num_days();
        let mut ret = ago_num.abs().to_string();
        ret.push_str(" day");
        if ago_num != 1 && ago_num != -1 {
            ret.push_str("s");
        }
        if ago_num < 0 {
            ret.push_str(" AHEAD");
        } else {
            ret.push_str(" ago")
        }
        ret
    }

    fn dialogue_new_session(&mut self) -> Result<bool, Error> {
        let mut session_date = self.db.select_current_date()?;
        loop {
            let inp = Tui::get_user_input("+ Session date: ");
            match inp.parse::<i32>() {
                Ok(i) => {
                    session_date = session_date - Duration::days(i.into());
                    break;
                },
                Err(_) => {
                    if inp == "" {
                        break;
                    } else if inp == "q" || inp == "c" {
                        return Ok(false);
                    }
                    // TODO Allow actual date as input, also HELP
                    println!("+ !!! Invalid input.");
                },
            }
        }
        
        let mut transaction = self.db.transaction_start()?;
        let new_session_id = Db::transaction_insert_session(&mut transaction, &session_date)?;
        let new_session_date = Db::transaction_select_session_date(&mut transaction, new_session_id)?.format("%d.%m.");
        println!("+ ... Creating session on {}", new_session_date);
        if Tui::add_lifts(&mut transaction, new_session_id)? == 0 {
            return Ok(false);
        }
        if Tui::get_user_input("+ Log session? ([YES]/cancel)") != "" {
            return Ok(false);
        }
        Db::transaction_commit(transaction)?;
        Ok(true)
    }

    fn add_lifts(transaction: &mut Transaction, session_id: i32) -> Result<i32, Error> {
        let mut added_lifts = 0;
        loop {
            if Tui::dialogue_new_lift(transaction, session_id)? {
                added_lifts += 1;
            } else {
                println!("+ ... Lift cancelled.");
            }
            let inp = Tui::get_user_input("+ Add more lifts ([YES]/calcel) ? ");
            if inp != "" {
                break;
            }
        }
        Ok(added_lifts)
    }

    fn dialogue_new_lift(transaction: &mut Transaction, session_id: i32) -> Result<bool, Error> {
        let exercises = Db::transaction_select_exercises(transaction)?;
        if exercises.len() == 0 {
            println!("+ !!! [No defined exercises]");
            return Ok(false);
        }

        let selected_exercise = match Tui::select_exercise(&exercises) {
            Some(exercise) => exercise,
            None => return Ok(false),
        };

        println!("+ ... Selected '{}'.", selected_exercise.1);

        let (weight_default, reps_default, sets_default) = match selected_exercise.0 {
            5 => {
                //Chinups
                (Some(0.0), None, Some(1.0))
            },
            6 => {
                // Clean
                (None, Some(1.0), Some(1.0))
            },
            10 => {
                // Snatch
                (None, Some(1.0), Some(1.0))
            },
            _ => {
                // Default
                (None, Some(5.0), Some(1.0))
            },
        };

        let weight_def_str = match weight_default {
            None => String::new(),
            Some(f) => format!(" ({})", f).to_string(),
        };
        let reps_def_str = match reps_default {
            None => String::new(),
            Some(f) => format!(" ({})", f).to_string(),
        };
        let sets_def_str = match sets_default {
            None => String::new(),
            Some(f) => format!(" ({})", f).to_string(),
        };

        let weight = match Tui::get_user_input_float(format!("+ Weight{}: ", weight_def_str).as_str(), weight_default) {
            Some(f) => f,
            None => return Ok(false),
        };
        let reps = match Tui::get_user_input_float(format!("+ Reps{}: ", reps_def_str).as_str(), reps_default) {
            Some(f) => f,
            None => return Ok(false),
        };
        let sets = match Tui::get_user_input_float(format!("+ Sets{}: ", sets_def_str).as_str(), sets_default) {
            Some(f) => f,
            None => return Ok(false),
        };

        Db::transaction_insert_lift(transaction, selected_exercise.0, session_id, weight, reps, sets)?;
        
        Ok(true)
    }

    fn select_exercise(exercises: &Vec<(i32, String)>) -> Option<(i32, String)> {
        loop {
            let inp = Tui::get_user_input("+ Exercise: ");
            if inp == "q" || inp == "c" {
                return None;
            }

            let possible_exercises: Vec<(i32, String)> = Tui::match_name_to_exercise(inp, &exercises);

            if possible_exercises.len() > 1 {
                println!("+ !!! Too many exercises match: {}", Tui::get_exercises_string(&possible_exercises));
                continue;
            }
            match possible_exercises.get(0) {
                Some(exercise) => return Some(exercise.clone()),
                None => {println!("+ !!! No matching exercises. Known exercises: {}", Tui::get_exercises_string(&exercises)); continue;},
            }
        }
    }

    fn get_exercises_string(exercises: &Vec<(i32, String)>) -> String {
        let mut ret = String::new();
        for (index, exercise) in exercises.iter().enumerate() {
            ret.push_str(exercise.1.as_str());
            if index < (exercises.len()-1) {
                ret.push_str("; ");
            }
        }
        ret
    }

    fn match_name_to_exercise(inp: String, exercises: &Vec<(i32, String)>) -> Vec<(i32, String)> {
        let mut ret: Vec<(i32, String)> = Vec::new();
        if inp == "" {
            return ret;
        }
        for exercise in exercises {
            if exercise.1.len() < inp.len() {
                continue;
            }
            if inp.to_lowercase() == &exercise.1.to_lowercase()[..inp.len()] {
                ret.push(exercise.clone());
            }
        }

        ret
    }

    fn dialogue_get_pr(&mut self) -> Result<bool, Error> {
        let exercises = self.db.select_exercises()?;
        if exercises.len() == 0 {
            println!("+ !!! [No defined exercises]");
            return Ok(true);
        }

        let selected_exercise = match Tui::select_exercise(&exercises) {
            Some(exercise) => exercise,
            None => return Ok(false),
        };

        println!("+ ... Selected '{}'.", selected_exercise.1);

        let reps_default = match selected_exercise.0 {
            5 => {
                //Chinups
                None
            },
            6 => {
                // Clean
                Some(1.0)
            },
            10 => {
                // Snatch
                Some(1.0)
            },
            _ => {
                // Default
                Some(5.0)
            },
        };

        let reps_def_str = match reps_default {
            None => String::new(),
            Some(f) => format!(" ({})", f).to_string(),
        };

        let reps = match Tui::get_user_input_float(format!("+ Reps{}: ", reps_def_str).as_str(), reps_default) {
            Some(f) => f,
            None => return Ok(false),
        };

        let pr_weight = match self.db.select_exercise_weight_pr(selected_exercise.0, reps) {
            Ok(w) => w,
            Err(_) => {println!("[No such lifts found.]"); return Ok(true);}
        };

        println!("{}: {}x{}", selected_exercise.1, pr_weight, reps);
        
        Ok(true)
    }

    fn dialogue_add_exercise(&mut self) -> Result<bool, Error> {
        let exercise_name = Tui::get_user_input("Exercise name: ");
        if exercise_name == "q" || exercise_name == "c" {
            return Ok(false);
        }
        _ = self.db.insert_exercise(exercise_name)?;
        Ok(true)
    }

}

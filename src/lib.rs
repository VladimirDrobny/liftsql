use postgres::{Client, NoTls, Error, Transaction};
use chrono::NaiveDate;

pub struct Db {
    client: Client,
}

impl Db {
    pub fn new(host: &str, user: &str, password: &Option<&str>, dbname: &str) -> Result<Db, Error> {
        let mut client = Db::connect(host, user, password, &Some(dbname));
        if let Err(err) = &client {
            if let Some(code) = err.code() {
                if code.code() == "3D000" {
                    println!("Database '{}' not found. Creating one...", dbname);
                    client = Db::init_database(host, user, password, dbname);
                }
            }
        }

        Ok(Db {
            client: client?,
        })
    }

    fn connect(host: &str, user: &str, password: &Option<&str>, dbname: &Option<&str>) -> Result<Client, Error> {
        let mut params: String = String::new();
        params.push_str("host=");
        params.push_str(host);
        params.push_str(" user=");
        params.push_str(user);
        if let Some(pass) = password {
            params.push_str(" password=");
            params.push_str(pass);
        }
        if let Some(dbn) = dbname {
            params.push_str(" dbname=");
            params.push_str(dbn);
        }

        Client::connect(&params, NoTls)
    }

    fn init_database(host: &str, user: &str, password: &Option<&str>, dbname: &str) -> Result<Client, Error> {
        let mut client = Db::connect(host, user, password, &None)?;
        let mut create_db_query = String::from("create database ");
        create_db_query.push_str(dbname);
        create_db_query.push_str(";");
        client.execute(create_db_query.as_str(), &[])?;
        client  = Db::connect(host, user, password, &Some(dbname))?;
        
        client.execute("CREATE TABLE exercises (id SERIAL, name TEXT NOT NULL, PRIMARY KEY (id));", &[])?;
        client.execute("CREATE TABLE sessions (id SERIAL, date DATE NOT NULL, PRIMARY KEY (id));", &[])?;
        client.execute("CREATE TABLE lifts (id SERIAL, exercise_id INT NOT NULL, session_id INT NOT NULL, weight REAL NOT NULL, reps REAL NOT NULL, sets REAL NOT NULL, PRIMARY KEY (id));", &[])?;
        //client.execute("CREATE TABLE comments (id SERIAL, session_id INT NOT NULL, comment_text_id INT NOT NULL, PRIMARY KEY (id));", &[])?;
        //client.execute("CREATE TABLE comment_text(id SERIAL, text TEXT NOT NULL, PRIMARY KEY (id));", &[])?;

        let exercises = ["Squat", "Bench", "Deadlift", "Press", "Chinups", "Clean", "Lat pulldowns", "Front squat", "Rows", "Snatch"];
        for exercise in exercises {
            client.execute("INSERT INTO exercises (name) values ($1);", &[&exercise])?;
        }

        Ok(client)
    }

    pub fn select_current_date(&mut self) -> Result<NaiveDate, Error> {
        match self.client.query_one("SELECT CURRENT_DATE;", &[]) {
            Ok(row) => Ok(row.get(0)),
            Err(err) => Err(err),
        }
    }

    pub fn select_session_date(&mut self, id: i32) -> Result<NaiveDate, Error> {
        match self.client.query_one("SELECT date FROM sessions WHERE id=$1;", &[&id]) {
            Ok(row) => Ok(row.get(0)),
            Err(err) => Err(err),
        }
    }

    pub fn select_last_session_id(&mut self) -> Result<i32, Error> {
        match self.client.query_one("SELECT id FROM sessions ORDER BY date DESC LIMIT 1;", &[]) {
            Ok(row) => Ok(row.get(0)),
            Err(err) => Err(err),
        }
    }

    pub fn insert_session(&mut self, date: &NaiveDate) -> Result<i32, Error> {
        match self.client.query_one("INSERT INTO sessions (date) VALUES ($1) returning id;", &[&date]) {
            Ok(row) => Ok(row.get(0)),
            Err(err) => Err(err),
        }
    }

    pub fn select_exercise_name(&mut self, exercise_id: i32) -> Result<String, Error> {
        match self.client.query_one("SELECT name FROM exercises WHERE id=$1;", &[&exercise_id]) {
            Ok(row) => Ok(row.get(0)),
            Err(err) => Err(err),
        }
    }

    pub fn select_exercise_weight_pr(&mut self, exercise_id: i32, reps: f32) -> Result<f32, Error> {
        match self.client.query_one("SELECT weight FROM lifts WHERE exercise_id=$1 AND reps=$2 ORDER BY weight DESC LIMIT 1;", &[&exercise_id, &reps]) {
            Ok(row) => Ok(row.get(0)),
            Err(err) => Err(err),
        }
    }

    pub fn select_exercise_reps_pr(&mut self, exercise_id: i32, weight: f32) -> Result<f32, Error> {
        match self.client.query_one("SELECT reps FROM lifts WHERE exercise_id=$1 AND weight=$2 ORDER BY reps DESC LIMIT 1;", &[&exercise_id, &weight]) {
            Ok(row) => Ok(row.get(0)),
            Err(err) => Err(err),
        }
    }

    pub fn transaction_start(&mut self) -> Result<Transaction, Error> {
        self.client.transaction()
    }

    pub fn transaction_commit(transaction: Transaction) -> Result<(), Error> {
        transaction.commit()
    }

    pub fn transaction_insert_session(transaction: &mut Transaction, date: &NaiveDate) -> Result<i32, Error> {
        match transaction.query_one("INSERT INTO sessions (date) VALUES ($1) returning id;", &[&date]) {
            Ok(row) => Ok(row.get(0)),
            Err(err) => Err(err),
        }
    }

    pub fn transaction_select_session_date(transaction: &mut Transaction, id: i32) -> Result<NaiveDate, Error> {
        match transaction.query_one("SELECT date FROM sessions WHERE id=$1;", &[&id]) {
            Ok(row) => Ok(row.get(0)),
            Err(err) => Err(err),
        }
    }

    pub fn transaction_select_exercises(transaction: &mut Transaction) -> Result<Vec<(i32, String)>, Error> {
        let query = transaction.query("SELECT id, name FROM exercises;", &[])?;
        let mut ret: Vec<(i32, String)> = Vec::new();
        for exercise in query {
            ret.push((exercise.get(0), exercise.get(1)));
        }
        Ok(ret)
    }

    pub fn transaction_insert_lift(transaction: &mut Transaction, exercise_id: i32, session_id: i32, weight: f32, reps: f32, sets: f32) -> Result<i32, Error> {
        match transaction.query_one("INSERT INTO lifts (exercise_id, session_id, weight, reps, sets) VALUES ($1, $2, $3, $4, $5) returning id;", &[&exercise_id, &session_id, &weight, &reps, &sets]) {
            Ok(row) => Ok(row.get(0)),
            Err(err) => Err(err),
        }
    }
}

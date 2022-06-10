use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use rocket::http::Status;
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::State;
#[macro_use]
extern crate rocket;
extern crate rand;

struct Database {
    teams: HashMap<usize, Team>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
struct Member {
    id: usize,
    name: String,
    age: u8,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
struct Team {
    members: Vec<Member>,
}

static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[post("/team", format = "json", data = "<team>")]
async fn create_team(db: &State<Mutex<Database>>, team: Json<Team>) -> Status {
    let id = ID_COUNTER.fetch_add(1, Ordering::Relaxed);

    db
        .lock()
        .ok()
        .and_then(|mut table| table.teams.insert(id, team.into_inner()));

    Status::Ok
}

#[get("/team/<id>")]
async fn fetch_team(db: &State<Mutex<Database>>, id: usize) -> Option<Json<Team>> {
    let result = db
        .lock()
        .ok()
        .and_then(|table| table.teams.get(&id).cloned());

    result.map(|team| Json(team))
}

#[post("/team/<id>", format = "json", data = "<member>")]
async fn add_member(db: &State<Mutex<Database>>, id: usize, member: Json<Member>) -> Status {
    let result = db.lock().ok().and_then(|mut table| {
        table
            .teams
            .get_mut(&id)
            .map(|team| team.members.push(member.into_inner()))
    });

    match result {
        Some(_) => Status::Ok,
        None => Status::BadRequest,
    }
}

#[delete("/team/<id>/members/<member_id>")]
async fn remove_member(db: &State<Mutex<Database>>, id: usize, member_id: usize) -> Status {
    let result = db.lock().ok().map(|mut table| {
        let maybe_team = table.teams.get(&id);

        let filtered_members = match maybe_team {
            Some(team) => team
                .members
                .iter()
                .filter_map(|m| {
                    if m.id != member_id {
                        Some(m.clone())
                    } else {
                        None
                    }
                })
                .collect(),
            None => Vec::new(),
        };

        table.teams.insert(id, {Team {
            members: filtered_members
        }});
    });

    match result {
        Some(_) => Status::Ok,
        None => Status::BadRequest,
    }
}

#[launch]
async fn rocket() -> _ {
    let db = Mutex::new(Database {
        teams: HashMap::new(),
    });

    rocket::build()
        .manage(db)
        .mount("/", routes![fetch_team, create_team, add_member, remove_member])
}

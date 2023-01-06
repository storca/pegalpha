use rocket_db_pools::{sqlx};
use rocket_db_pools::sqlx::Row;
use rocket_db_pools::sqlx::mysql::MySqlConnection;

use crate::defs::*;
use crate::config;

pub async fn retrieve_attendee(db: &mut MySqlConnection, order_ref:&str) -> Option<IdentifiedAttendee> {
    if order_ref.len() > 10
    {
        return None;
    }
    else {
        let iter = order_ref.split('-');
        let split_ref = iter.collect::<Vec<&str>>();
        if split_ref.len() != 2 {
            return None;
        }
        // println!("order_ref : {}, index : {}", split_ref[0], split_ref[1]);
        // Retrieve attendee_id, ticket id and gender (one row only)
        let attendee_stmt = format!("SELECT a.id, a.ticket_id, qa.answer_text
        FROM orders o, attendees a, question_answers qa
        WHERE o.event_id = 2
        AND qa.attendee_id = a.id
        AND qa.question_id = {}
        AND a.is_cancelled = 0
        AND a.ticket_id IN {}
        AND o.id = a.order_id
        AND o.order_reference = ?
        AND a.reference_index = ?", config::get_option("gender_question_id"), config::get_option("pack_ticket_ids"));

        let attendee_res = sqlx::query(&attendee_stmt).bind(split_ref[0]).bind(split_ref[1])
        .fetch_optional(&mut *db).await;

        if attendee_res.is_err() {
            panic!("SQL error while retrieving attendee");
        }

        let first_row_opt = attendee_res.unwrap();
        if first_row_opt.is_none() {
            return None;
        }

        let first_row = first_row_opt.unwrap();

        let attendee_id:u32 = first_row.get(0);
        let ticket_id:u32 = first_row.get(1);
        
        let gender_name:String = first_row.get(2);
        let gender:AttendeeGender;

        match gender_name.as_str() {
            "Male" => gender = AttendeeGender::M,
            "Female" => gender = AttendeeGender::F,
            _other => panic!("Gender name unknown \'{}\'", _other)
        }

        // Get attendee sports
        // Ensure the correct sports are made available
        let sport_question_ids:String;

        match gender {
            AttendeeGender::M => sport_question_ids = config::get_option("male_sport_question_ids"),
            AttendeeGender::F => sport_question_ids = config::get_option("female_sport_question_ids")
        }

        let sports_stmt = format!(
            "SELECT answer_text FROM question_answers
            WHERE attendee_id = ?
            AND question_id IN {}", sport_question_ids);

        let sports_fut = sqlx::query(&sports_stmt).bind(attendee_id)
        .fetch_all(&mut *db);

        let sports_res = sports_fut.await;
        if sports_res.is_err() {
            panic!("SQL error while retrieving attendee sports");
        }

        let mut sports:Vec<Sport> = Vec::new();

        for row in sports_res.unwrap() {
            let sport_name:String = row.get(0);
            
            let sport = config::find_sport(sport_name.as_str(), Some(gender));
            if sport.is_some() { //Ignore sports that are not in the config file (individual sports)
                sports.push(sport.unwrap());
            }
        }

        let school_stmt: String = format!(
            "SELECT qo.id FROM question_options qo
            JOIN question_answers qa ON qa.question_id = qo.question_id
            WHERE qa.question_id = {} AND qa.attendee_id = ? AND qo.name = qa.answer_text",
            config::get_option("school_question_id")
        );

        let school_fut = sqlx::query(&school_stmt).bind(attendee_id)
        .fetch_one(&mut *db);
        let school_res = school_fut.await;

        let school_id: u32;
        if school_res.is_err() {
            panic!("SQL error while retrieving attendee school");
        }
        else {
            school_id = school_res.unwrap().get(0);
        }

        return Some(IdentifiedAttendee { 
            id: attendee_id, 
            ticket_id: ticket_id, 
            gender: gender, 
            sports: sports,
            school_id: school_id});
    }
}

pub fn has_sport(attendee:&IdentifiedAttendee, sport_name:&str) -> bool {
    for sport in &attendee.sports {
        if sport.name.as_str() == sport_name {
            return true;
        }
    }
    return false;
}

pub fn has_correct_gender(attendee:&IdentifiedAttendee, team_sport:&Sport) -> bool {
    for sport in &attendee.sports {
        if sport.gender == team_sport.gender {
            return true;
        }
    }
    return false;
}

pub async fn has_team(db: &mut MySqlConnection, attendee:&IdentifiedAttendee, sport: &str) -> bool {
    let row = sqlx::query("SELECT t.id, t.name
    FROM teams t, team_members tm
    WHERE tm.team_id = t.id
    AND tm.attendee_id = ?
    AND t.sport = ?").bind(attendee.id).bind(sport)
    .fetch_optional(db).await;

    match row {
        Ok(o) => 
            match o {
                Some(_r) => false,
                None => true
            },
        Err(e) => panic!("MySQL error during team registration test : {e:?}")
    }
}

pub async fn validate_attendee(db: &mut MySqlConnection, attendee:&IdentifiedAttendee, sport: &Sport) -> AttendeeStatus {
    let athlete_tickets = config::get_option("athlete_ticket_ids");
    let mut is_an_athlete = false;
    for ticket_id in athlete_tickets.split(',') {
        let id = ticket_id.parse::<u32>().unwrap();
        if id == attendee.ticket_id {
            is_an_athlete = true;
        }
    }

    // Check if attendee sports are valid
    if attendee.sports.len() == 0 {
        AttendeeStatus::InvalidSport
    }
    else if !is_an_athlete {
        AttendeeStatus::NotAnAthlete
    }
    // Check if attendee is already in a team
    else if has_team(db, attendee, sport.name.as_str()).await {
        AttendeeStatus::AlreadyInATeam
    }
    else if !has_sport(attendee, sport.name.as_str()){
        AttendeeStatus::SportNotRegistered
    }
    else if !has_correct_gender(attendee, sport) {
        AttendeeStatus::InvalidGender
    }
    else {
        AttendeeStatus::Ok
    }
}
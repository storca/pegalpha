use std::env;
use ini::Ini;
use crate::defs::*;

pub fn get_option(opt_name: &str) -> String {
    let filename = env::var("EAG_API_CONFIG");
    let i: Ini;
    if filename.is_err() {
        i = Ini::load_from_file("sample.conf").unwrap();
    }
    else {
        i = Ini::load_from_file(filename.unwrap()).unwrap();
    }
    for(sec, prop) in i.iter() {
        if sec.unwrap() == "main" {
            let opt = prop.get(opt_name);
            if opt.is_some() {
                let mut s = String::new();
                s.push_str(opt.unwrap());
                return s;
            }
            else {
                panic!("Missing option in configuration file under section [main]: {}", opt_name);
            }
        }
    }
    panic!("Missing section [main] in configuration file");
}

pub fn find_sport(sport: &str, gender:Option<AttendeeGender>) -> Result<Sport, String> {
    let filename = env::var("EAG_API_CONFIG");
    let i: Ini;
    if filename.is_err() {
        i = Ini::load_from_file("sample.conf").unwrap();
    }
    else {
        i = Ini::load_from_file(filename.unwrap()).unwrap();
    }
    for (sec, prop) in i.iter() {
        let section_name = sec.unwrap();
        if section_name == sport {
            // Check if sport type is strict or mixed
            let sport_type = prop.get("gender");
            if sport_type.is_some() {
                let max_teams_per_school:u8;
                let mut school_mix_allowed:bool = false;

                match prop.get("max_teams_per_school") {
                    Some(o) => max_teams_per_school = o.parse().unwrap(),
                    None => return Err(format!("Missing max_teams_per_school in [{section_name}]"))
                }

                match prop.get("school_mix_allowed") {
                    Some(o) => school_mix_allowed = o.parse().unwrap(),
                    None => ()
                }

                // Does the sport support mixed teams or strict teams ?
                match sport_type.unwrap() {
                    //This sport supports mixed teams
                    "mixed" => {
                        let min_opt = prop.get("min");
                        let max_opt = prop.get("max");
                        if min_opt.is_some() && max_opt.is_some() { //if min and max are both defined
                            // convert min and max to i8 and return a structure
                            let s = Sport {
                                name: String::from(section_name),
                                min_players: min_opt.unwrap().parse::<u8>().unwrap(),
                                max_players: max_opt.unwrap().parse::<u8>().unwrap(),
                                gender: SportGender::Mixed,
                                max_teams_per_school: max_teams_per_school,
                                school_mix_allowed: school_mix_allowed
                            };
                            return Ok(s);
                        }
                        else {
                            return Err(format!("Missing fields under [{section_name}], it sould include fields \'min\' and \'max\'"));
                        }
                    }
                    //This sport supports only one gender per team
                    "strict" => {
                        let min_m_opt = prop.get("minM");
                        let max_m_opt = prop.get("maxM");
                        let min_f_opt = prop.get("minF");
                        let max_f_opt = prop.get("maxF");
                        if min_m_opt.is_none() || max_m_opt.is_none() || min_f_opt.is_none() || max_f_opt.is_none() {
                            return Err(format!("Missing fields under [{section_name}], it sould include fields \'minM\', \'maxM\', \'maxF\' and \'maxF\'"))
                        }
                        if gender.is_some() {
                            match gender.unwrap() {
                                AttendeeGender::M => {
                                    return Ok(Sport { name: String::from(section_name),
                                                        min_players: min_m_opt.unwrap().parse::<u8>().unwrap(), 
                                                        max_players: max_m_opt.unwrap().parse::<u8>().unwrap(), 
                                                        gender: SportGender::M,
                                                        max_teams_per_school: max_teams_per_school,
                                                        school_mix_allowed: school_mix_allowed});
                                },
                                AttendeeGender::F => {
                                    return Ok(Sport { name: String::from(section_name), 
                                        min_players: min_f_opt.unwrap().parse::<u8>().unwrap(), 
                                        max_players: max_f_opt.unwrap().parse::<u8>().unwrap(), 
                                        gender: SportGender::F,
                                        max_teams_per_school: max_teams_per_school,
                                        school_mix_allowed: school_mix_allowed });
                                }
                            }   
                        }
                        else {
                            return Err(format!("Sport {section_name} is strict and attendee gender is required"));
                        }
                    }
                    // When type option is not valid 
                    _other => {
                        return Err(format!("Invalid sport type under [{section_name}], is has to be either \'mixed\' or \'strict\' : \'{_other}\' is invalid"))
                    } 
                }
            }
        }
    }
    return Err(String::from("Unknown error"));
}
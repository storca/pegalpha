use std::env;
use ini::Ini;
use rocket::log::private::warn;

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

pub fn find_sport(sport: &str, gender:Option<AttendeeGender>) -> Option<Sport> {
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
                                gender: SportGender::Mixed
                            };
                            return Some(s);
                        }
                        else {
                            warn!("Missing fields under [{}], it sould include fields \'min\' and \'max\'", section_name);
                            return None;
                        }
                    }
                    //This sport supports only one gender per team
                    "strict" => {
                        let min_m_opt = prop.get("minM");
                        let max_m_opt = prop.get("maxM");
                        let min_f_opt = prop.get("minF");
                        let max_f_opt = prop.get("maxF");
                        if min_m_opt.is_none() || max_m_opt.is_none() || min_f_opt.is_none() || max_f_opt.is_none() {
                            warn!("Missing fields under [{}], it sould include fields \'minM\', \'maxM\', \'maxF\' and \'maxF\'", section_name);
                        }
                        if gender.is_some() {
                            match gender.unwrap() {
                                AttendeeGender::M => {
                                    return Some(Sport { name: String::from(section_name),
                                                        min_players: min_m_opt.unwrap().parse::<u8>().unwrap(), 
                                                        max_players: max_m_opt.unwrap().parse::<u8>().unwrap(), 
                                                        gender: SportGender::M });
                                },
                                AttendeeGender::F => {
                                    return Some(Sport { name: String::from(section_name), 
                                        min_players: min_f_opt.unwrap().parse::<u8>().unwrap(), 
                                        max_players: max_f_opt.unwrap().parse::<u8>().unwrap(), 
                                        gender: SportGender::F });
                                }
                            }   
                        }
                        else {
                            warn!("Sport {} is strict and attendee gender is required", section_name);
                        }
                    }
                    // When type option is not valid 
                    _other => {
                        warn!("Invalid sport type under [{}], is has to be either \'mixed\' or \'strict\' : \'{}\' is invalid", section_name, _other);
                    } 
                }
            }
        }
    }
    return None;
}
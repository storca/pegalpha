-- Up
CREATE TABLE teams(
    id INT(10) UNSIGNED NOT NULL AUTO_INCREMENT,
    school_id INT(10) UNSIGNED NOT NULL,
    name VARCHAR(256),
    captain_id INT(10) UNSIGNED NOT NULL,
    uuid VARCHAR(36) NOT NULL DEFAULT UUID(),
    sport VARCHAR(32) NOT NULL,
    gender VARCHAR(32) NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (school_id) REFERENCES question_options(id),
    FOREIGN KEY (captain_id) REFERENCES attendees(id)
) ENGINE=INNODB;

CREATE TABLE team_members(
    id INT(10) UNSIGNED NOT NULL AUTO_INCREMENT,
    team_id INT(10) UNSIGNED NOT NULL,
    attendee_id INT(10) UNSIGNED NOT NULL,
    PRIMARY KEY(id),
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
    FOREIGN KEY (attendee_id) REFERENCES attendees(id)
) ENGINE=INNODB;

-- Down
DROP TABLE teams;
DROP TABLE team_members;
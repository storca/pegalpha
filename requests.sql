-- Check for existing team in a given school with attendee_id and sport

SELECT t.name FROM teams t
JOIN question_options qo ON qo.id = t.school_id
JOIN question_answers qa ON qa.question_id = qo.question_id
WHERE qa.attendee_id = ? AND t.sport = ?
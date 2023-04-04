# pegalpha

Système de gestion des équipes sportives et de check-in.

Développé à l'occasion des European Aerostudent Games 2023.

![landing page](https://github.com/storca/pegalpha/blob/master/screenshots/welcome.png?raw=true)

## Description

Ce projet est une application web qui à l'origine, se chargait de vérifier et d'inscrire les équipes aux EAG.

Ce site web a été programmé en Python pour l'édition 2022 avec fastapi et flask, le problème était que l'inscription d'une équipe était très lente (pouvait prendre jusqu'à 10s). Cette année avons aussi manqué de flexibilité avec l'ancienne API.

J'ai donc décidé de la reprogrammer entièrement en Rust avec le framework Rocket pour améliorer ses performances et réduire la durée du déboguage.
Ce projet a progressivement évolué d'une simple API vers une appli web complète, grâce aux possibilités de templating Tera de Rocket.

Cette API fonctionne par défaut avec la base de données de Attendize, à laquelle on ajoute une table 'teams' (cf migration.sql).

La vérification et l'inscription des équipes n'est pas triviale puisque il y a différentes règles de composition des équipes différentes selon les sports.

## Fonctionnalités
* Fonctionne avec Attendize
* Configuration des règles de composition pour chaque sport
* Inscription et contrôle des équipes
* Listage des équipes, des membres par équipe
* Listage des membres par sport sans équipe
* Export d'une équipe en PDF monochrome (wkhtmltopdf)
* Check-in augmenté (couleurs des bracelets à mettre, tickets repas à donner)

## Règles de composition à configurer pour chaque sport

* Mixité ou non du sport (strict ou mixed)
* Intervalle du nombre de joueurs acceptables (différent selon si le sport est strict ou mixte, obligatoire)
* Possibilité de limiter le nombre d'équipes par école pour un sport donné (obligatoire)
* Possibilité d'autoriser ou d'interdire des membres venant d'écoles différentes dans la même équipe, pour chaque sport (non par défaut, facultatif)

### Exemple de configuration

```
[main]
athlete_ticket_ids = 1,2,3,4,5,7,9,12
male_sport_question_ids = (5, 6, 8)
female_sport_question_ids = (5, 6, 7, 8)
gender_question_id = 17
school_question_id = 15
sport_secret = secret1
check_in_secret = secret2
check_in_read_only = false
team_registration_open = true

[Football]
gender = strict
max_teams_per_school = 4
minM = 8
maxM = 12
minF = 8
maxF = 11
school_mix_allowed = true

[Swimming]
gender = mixed
max_teams_per_school = 4
min = 4
max = 4
```

## RETEX

Ce projet a l'avantage d'automatiser beaucoup de vérifications que le pôle sport aurait dû effectuer à la main.

Ca a été l'occasion aussi de découvrir Rust, un language compliqué mais qui fait gagner beaucoup de temps (98% de programmation, 2% de déboguage).

*On peut voir la progression de mon niveau de Rust, il y a des passages programmés qui fonctionnent, mais qui n'ont pas été programmés dans l'état d'esprit de Rust.*

J'ai fais le choix de ne pas utiliser un ORM et de rester avec une bibliothèque SQL simple pour ne pas complexifier un projet qui l'était déjà. J'estimais qu'apprendre Rust, Rocket et Diesel en même temps faisait beaucoup, décision discutable.

### Critiques

Un des problèmes est que je me suis trouvé à de nombreuses reprises "forcé" de développer des fonctionnalités additionelles face à des demandes (justifiées) que j'ai eu de la part du pôle sport.

On a l'impression qu'en automatisant certaines choses on gagne du temps, mais au total le gain de temps a été négligeable.

En prenant du recul, il aurait été plus simple de laisser le pôle sport composer les équipes, en leur donnant la liste des participants par école, en les laissant échanger avec les responsables de délégation. Une communication directe entre le pôle sport et les délégations aurait été un gain de temps non négligeable.

Utiliser un ORM aurait pu être un gain de temps.

L'organisation du code pourrait être améliorée.

## Captures d'écran
### Landing page

![landing page](https://github.com/storca/pegalpha/blob/master/screenshots/welcome.png?raw=true)

### Composition d'une équipe

![team composition](https://github.com/storca/pegalpha/blob/master/screenshots/compose.png?raw=true)

### Succès d'inscription d'une équipe (animé)

![team registration success](https://github.com/storca/pegalpha/blob/master/screenshots/team-success.png?raw=true)

### Liste des équipes

![team list](https://github.com/storca/pegalpha/blob/master/screenshots/team-list.png?raw=true)

### Liste des membres d'une équipe

Omis, pour des raisons évidentes de confidentialité.

### Liste des sans-équipe, par sport

![no team](https://github.com/storca/pegalpha/blob/master/screenshots/no-team-list.png?raw=true)

### Résultat de check-in

![check in result](https://github.com/storca/pegalpha/blob/master/screenshots/check-in.png?raw=true)

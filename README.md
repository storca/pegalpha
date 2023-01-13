# European Aerostudent Games
## Système d'inscription et de contrôle des équipes

### Description

Ce projet est une API REST HTTP qui se charge de vérifier et d'inscrire les équipes aux EAG.

Cette API a été programmée en Python pour l'édition 2022 avec fastapi, le problème était que l'inscription d'une équipe était très lente (pouvait prendre jusqu'à 10s). Cette année nous allons aussi manquer de flexibilité avec l'ancienne API.

J'ai donc décidé de la reprogrammer entièrement en Rust avec le framework Rocket pour améliorer ses performances et réduire la durée du déboguage.

Cette API fonctionne par défaut avec la base de données de Attendize, à laquelle on ajoute une table 'teams'.

La vérification et l'inscription des équipes n'est pas triviale puisque il y a différentes règles de composition des équipes différentes selon les sports.

### Fonctionnalités

* Possibilité de changer la configuration pour les requêtes MYSQL (IDs de question notemment)
* Configuration des règles de chaque sport
* Lecture de la configuration "at-runtime"

### Règles de composition à configurer pour chaque sport

* Mixité ou non du sport
* Intervalle du nombre de joueurs acceptables (peut varier selon si l'équipe est masculine ou féminine dans le cas d'un sport strict)
* Possibilité d'autoriser ou d'interdire plusieurs équipes par école selon le sport

### Exemple de configuration

```
[Rugby]
type = strict
allow_multiple_teams = no
minM = 14
maxM = 16
minF = 12
maxF = 14

[Handball]
type = mixed
allow_multiple_teams = no
min = 14
max = 16
```
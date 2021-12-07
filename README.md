# OSM-Firefighter
Eine Projektarbeit Softwaretechnik an der Universität Stuttgart von Aimn Ahmed, Samuel Holderbach und Dominik Krenz. Betreut von Tobias Rupp und geprüft von Prof. Dr. Stefan Funke.

## Projektaufbau

Dieses Projekt besteht aus folgenden Komponenten:

* frontend: Ein Angular Projekt, dass die Benutzeroberfläche des Webservices beinhaltet (Docker Container Name: osm-ff-frontend)
* backend: Ein Rust Projekt welches die eigentliche Logik des Firefighter Problems und seine Strategien beinhaltet (Docker Container Name: osm-ff-backend)
* graphs: Ein Verzeichnis welches die Graphendateien beinhaltet die in der Docker Umgebung verwendet werden
* docker-compose.yml: Ein Docker Compose file welches dazu konfiguriert wurde um die beiden Services zu starten und diese miteinander Kommunizieren lassen

## Setup

### Lokaler Modus

Der Lokale Modus wird nur empfohlen wenn an diesem Projekt gearbeitet wird, da die Performanz des Backends im release Modus weitaus besser ist. 

Vorraussetzungen:

* node-js und Angular für das frontend
    * node-js: https://nodejs.org/en/download/current/
    * Angular: npm install -g @angular/cli
* rustup für das Backend (https://www.rust-lang.org/learn/get-started)

Backend:

Entweder über die Runconfigurations für IntelliJ (benötigt Plugin) oder über:

``cargo run data/``

Das Backend ist dann über den Port 8080 erreichbar.

Frontend:

Vor dem ersten Start müssen zunächst alle dependencies installiert werden mit:

``npm install``

Im Anschluss kann das Frontend gestartet werden über:

``ng serve``

Das frontend läuft nun auf dem Port 4200 und versucht mit einem Backend zu kommunizeren, das unter http://localhost:8080 erreichbar ist.

### Starten über Docker

Vorraussetzungen:

* Docker installiert
* graphs Verzeichnis beinhaltet Graphendateien

Dokumentation zur installation für Docker:

* Windows: https://docs.docker.com/desktop/windows/install/
* Linux: https://docs.docker.com/engine/install/#server (select your installed Linux distribution)

Zu den Graphdateien:

Beispielgraphen lassen sich aus dem Verzeichnis /backend/data kopieren. Wichtig ist, dass für jede Datei mit der Endung .fmi eine .ch.hub Datei des selben Namens existiert. Diese Datei beinhaltet die generierten Hub-Labels des Graphen.

Falls ein Graph verändert oder hinzugefügt wurde, muss der backend-container neu gestartet werden.

Starten des compose files:

Ist docker installiert und das grahps/ Verzeichnis beinhaltet Graph-Dateien kann das compose file gestartet werden über:

``docker compose up``

Durch diesen Befehl werden bestehende Docker image benutzt. Falls keine existieren werden diese automatisch gebaut. Die images können explizit gebaut werden mit:

``docker compose build``

Das Bauen der images kann einige Minuten dauern (ca 10 min).
Wurde das compose-file gestartet ist das frontend über den Port 80 erreichbar und das backend über den Port 8080.
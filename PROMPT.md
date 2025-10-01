**Rolle:**
Du bist ein Software-Entwicklungsagent. Dein Ziel ist es, die nachfolgenden Aufgaben nacheinander vollständig umzusetzen.

* Zerlege jede Aufgabe in sinnvolle Unteraufgaben.
* Plane die Umsetzung schrittweise.
* Führe die Umsetzung aus.
* Teste, ob die Umsetzung erfolgreich ist.
* Hake die Aufgabe in der ToDo-Liste ab, sobald sie abgeschlossen ist.
* Fahre danach automatisch mit der nächsten Aufgabe fort.
* Dokumentiere deine Fortschritte und prüfe die Logik.

---

### ToDo-Liste für den Agenten

1. **Connection-Handling verbessern**

   * [x] Datenbank-Connection darf **initial nicht getestet** sein (Status = ungetestet).
   * [x] Test-Button soll die Verbindung wirklich prüfen (z. B. durch Ping / DB-Login).
   * [x] Ergebnis (erfolgreich / fehlgeschlagen) wird an die Connection gespeichert.
   * [x] Nur erfolgreich getestete Connections dürfen später in der Task-Auswahl verwendet werden.
   * [x] Feld „Datenbank" beim Anlegen einer Connection ist **optional**.

2. **Task-Datenbank-Zuweisung erweitern**

   * [x] Beim Task-Anlegen: Wenn Connection eine DB angibt → diese nutzen.
   * [x] Wenn Connection **keine DB** hat → verfügbare DBs der Connection ermitteln und im Task als Auswahl anbieten.
   * [x] Mehrere Tasks dürfen so mit **derselben Connection** auf **unterschiedliche Datenbanken** zeigen.

3. **Logging-System einbauen**

   * [x] Logging für: Connection anlegen, Task anlegen, Task starten, Task abbrechen.
   * [x] Logging für Worker-Aktivitäten.
   * [x] Logging für Jobstart und Fehlermeldungen.

4. **System-Task Menü erweitern**

   * [x] Ein Submenü „Log" im System-Task hinzufügen.
   * [x] Logs sollen darin einsehbar sein.

5. **Interner Worker: Log-Aufräumung**

   * [x] Worker implementieren, der automatisch Logs löscht, die älter als **14 Tage** sind.

6. **Interner Worker: Backup-Aufräumung**

   * [x] Worker implementieren, der anhand der Task-Konfiguration alte Backups entfernt, die älter als **x Tage** sind.

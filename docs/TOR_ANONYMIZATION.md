# Anonymisation des Requêtes avec Tor

## Objectif
Assurer l'anonymat des requêtes sortantes effectuées par les plugins OSINT de Le Coq, en utilisant le réseau Tor comme proxy SOCKS5.

## Architecture concernée
Cette configuration s'applique aux composants suivants :
- **Plugins OSINT (Python)** : requêtes HTTP(S) vers des sources externes
- **Backend Rust (axum)** : communications réseau sécurisées
- **CLI Zig** : exports anonymisés (optionnel)

## Prérequis
- Tor installé sur le système hôte
- Python 3.11+
- Rust toolchain (pour backend)

## Installation de Tor

### Sur Ubuntu/Debian :
```bash
sudo apt update
sudo apt install tor
```

### Démarrer Tor :
```bash
sudo systemctl start tor
sudo systemctl enable tor  # (optionnel : démarrage automatique)
```

Tor écoute par défaut sur `127.0.0.1:9050` en SOCKS5.

---

## Configuration Python (Plugins OSINT)

### Installer PySocks :
```bash
pip install PySocks
```

### Exemple de plugin avec Tor :
```python
import socks
import socket
import requests

# Configurer SOCKS5 via Tor
socks.set_default_proxy(socks.SOCKS5, "127.0.0.1", 9050)
socket.socket = socks.socksocket

# Requête anonyme
response = requests.get("https://httpbin.org/ip")
print(response.json())  # Vérifier l'IP anonymisée
```

⚠️ **Note** : Tor ralentit les requêtes (10–30s par requête). À utiliser avec modération.

---

## Configuration Rust (Backend)

### Ajouter `reqwest` avec support SOCKS :
Dans `Cargo.toml` :
```toml
[dependencies]
reqwest = { version = "0.11", features = ["socks"] }
tokio = { version = "1", features = ["full"] }
```

### Exemple de requête anonyme :
```rust
use reqwest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all("socks5://127.0.0.1:9050")?)
        .build()?;

    let res = client.get("https://httpbin.org/ip").send().await?;
    println!("{}", res.text().await?);

    Ok(())
}
```

---

## Sécurité & Bonnes pratiques

### 1. **Pas de fuite DNS**
- Tous les plugins doivent utiliser le proxy Tor.
- Interdire les requêtes directes (firewall local).

### 2. **Chiffrement des données locales**
- PostgreSQL avec chiffrement TDE (Transparent Data Encryption)
- Clés de chiffrement gérées localement

### 3. **Minimiser les requêtes**
- Cache des réponses (TTL)
- Agrégation des requêtes (batch)

### 4. **Journalisation**
- Logs anonymisés
- Pas de stockage d'IP utilisateur

---

## Test de l’anonymat

### Vérifier l’IP de sortie :
```bash
curl --socks5 127.0.0.1:9050 https://httpbin.org/ip
```

### Vérifier les headers :
```bash
curl --socks5 127.0.0.1:9050 https://httpbin.org/headers
```

---

## Limitations

| Type | Détail |
|------|--------|
| **Performance** | Tor ralentit les requêtes |
| **Blocages** | Certains services bloquent Tor |
| **Usage intensif** | Risque de banissement temporaire |
| **Dépendance** | Tor doit tourner en arrière-plan |

---

## Références
- [Tor Project](https://www.torproject.org/)
- [PySocks Documentation](https://github.com/Anorov/PySocks)
- [reqwest with SOCKS](https://docs.rs/reqwest/latest/reqwest/)

---
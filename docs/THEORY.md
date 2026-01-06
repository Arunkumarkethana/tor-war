# 🧅 How Nipe & Tor Work: A Beginner's Guide

Welcome! If you are new to privacy tools, this guide will explain exactly what is happening "under the hood" when you use Nipe. We'll skip the complex math and use simple analogies.

---

## 1. The Problem: How the Normal Internet Works 🌐

Imagine sending a letter to your friend using the regular post service.
1. You write your **Return Address** (Your IP Address) on the envelope.
2. You write your **Friend's Address** (The Website's IP) on the envelope.
3. You hand it to the postman (Your ISP - Internet Service Provider).

**The Risks:**
*   **The Postman (ISP)** knows exactly who you are writing to. They can log this data and sell it or give it to governments.
*   **The Recipient (Website)** sees your Return Address. They know where you live (your location/country) and who you are.
*   **Intercepting spies** along the way can see who is talking to whom.

---

## 2. The Solution: Tor (The Onion Router) 🛡️

Tor is like a system of secret tunnels. Instead of handing your letter directly to the postman, you put your letter inside **three layers of locked boxes**.

### How it works (The 3-Hop System):

1.  **Entry Guard (Node 1)**:
    *   You send the locked box to Node 1.
    *   Node 1 peels off the first layer. It can verify *you* sent it, but it has no idea what's inside or where it's going next, other than "give this to Node 2".

2.  **Middle Relay (Node 2)**:
    *   Node 1 passes the box to Node 2.
    *   Node 2 peels the second layer. It doesn't know who you are (it only saw Node 1) and doesn't know the final destination. It just sees "give this to Node 3".

3.  **Exit Node (Node 3)**:
    *   Node 2 passes it to Node 3.
    *   Node 3 peels the final layer. It sees the actual letter and delivers it to the Website.
    *   **Crucial Point**: Node 3 knows *what* the message is (unless you use HTTPS), but it has **no idea who sent it**.

### Why "Onion"? 🧅
Because like an onion, your data is wrapped in layers of encryption. Each node peels off one layer, revealing only the next step. No single node knows the full path!

---

## 3. What Nipe Does (The "Transparent Proxy") 🔮

Tor normally only works for specific apps (like Tor Browser). If you open your Terminal, Spotify, or a game, they typically ignore Tor and connect directly, leaking your real IP.

**Nipe acts as a Traffic Cop for your entire computer.**

### The Architecture:

1.  **The Wall (Firewall)**:
    *   Nipe builds a digital wall around your computer using your system's firewall (`pf` on macOS, `iptables` on Linux, `netsh` on Windows).
    *   **The Rule**: "Nobody leaves this computer unless they go through the Tor door."

2.  **The Funnel (Redirection)**:
    *   When *any* app tries to access the internet, Nipe grabs that traffic and shoves it into the Tor network.
    *   This is called **Transparent Proxying**. You don't need to configure your apps; Nipe handles it for you.

3.  **The Kill Switch ☠️**:
    *   This is your safety net. If the Tor network crashes or your connection drops, Nipe instantly **locks the firewall**.
    *   **Result**: Your internet cuts off completely.
    *   **Why?** Typically, if a VPN fails, your computer silently switches back to your normal risky connection without telling you. Nipe prevents this "leak."

---

## 4. Key Concepts Explained

### 🔄 IP Rotation ("Ghost Mode")
Tor circuits (the path of 3 nodes) don't last forever.
*   **Tor**: Changes circuits every 10 minutes by default.
*   **Nipe**: Can force a change earlier (e.g., `nipe rotate`).
*   **Result**: One minute you appear to be in Germany 🇩🇪, the next minute in Japan 🇯🇵. This makes you incredibly hard to track over time.

### 🌉 Bridges (Bypassing Censorship)
Some countries (like China, Iran) or corporate firewalls block the list of known Tor Entry Nodes.
*   **The Fix**: "Bridges" are secret, unlisted Tor nodes.
*   **obfs4**: This is a technology that "obfuscates" (scrambles) your traffic so it looks like random noise. The firewall looks at it, thinks "this is just garbage data, not Tor," and lets it through.

### 🕵️ DNS Leaks
*   **DNS**: The phonebook of the internet (turning `google.com` into `142.250.x.x`).
*   **The Leak**: Sometimes, a VPN routes your *data* securely, but sends your *lookup requests* ("Where is google.com?") to your ISP insecurely.
*   **Nipe's Fix**: Nipe forces **UDP port 53** (DNS) traffic through Tor's special DNSPort. Your ISP doesn't even know *which* websites you are looking for.

### 🧪 Stream Isolation
Nipe configures Tor to use different circuits for different connections.
*   **Scenario**: You are logged into Facebook (Identity A) and browsing a news site (Identity B).
*   **Without Isolation**: Facebook might correlate your traffic.
*   **With Isolation**: Tor uses Path A for Facebook and Path B for the news site. They appear to come from different people.

---

## Summary

| Component | What it is | Metaphor |
|-----------|------------|----------|
| **Tor** | The Network | A system of secret tunnels. |
| **Nipe** | The Manager | The traffic cop that forces everyone into the tunnels. |
| **Circuit** | The Path | The specific route (Node 1 -> 2 -> 3) you are taking right now. |
| **Bridge** | The Secret Door | A hidden entrance to the tunnels if the main gate is blocked. |
| **Kill Switch** | The Emergency Brake | Stops everything if the tunnel collapses, so you don't get caught outside. |

Happy Hacking! 🕵️‍♂️

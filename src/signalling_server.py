import websockets
import asyncio
import json

peers = {}
relay_sessions = {}  # maps websocket -> peer websocket


def get_username_by_ws(ws):
    for username, info in peers.items():
        if info["websocket"] == ws:
            return username
    return None


async def signalling_handler(websocket):
    try:
        async for message in websocket:
            print("\n📡 Active users:", list(peers.keys()))

            if isinstance(message, bytes):
                # Relay binary message if in a relay session
                if websocket in relay_sessions:
                    peer_ws = relay_sessions[websocket]
                    await peer_ws.send(message)
                else:
                    print("⚠️ Received binary message but not in relay")
                continue

            data = json.loads(message)

            msg_type = data.get("type")

            if msg_type == "register":
                peers[data["username"]] = {
                    "ipv4_ip": data["ipv4_ip"],
                    "ipv4_port": data["ipv4_port"],
                    "ipv6_ip": data["ipv6_ip"],
                    "ipv6_port": data["ipv6_port"],
                    "password" : data["password"],
                    "websocket": websocket
                }
                await websocket.send(json.dumps({"status": "registered"}))
                print(f"✅ Registered: {data['username']}")

            if msg_type == "getusers":
                peers_cleaned = {
                    username : {
                        key:value
                        for key,value in info.items()
                        if key != "websocket"
                    }
                    for username, info in peers.items()
                }
                await websocket.send(json.dumps(peers_cleaned))

            elif msg_type == "initiate_relay":
                username = data.get("username")
                if username not in peers:
                    print("user not found\n")
                else:
                    peers[username]["websocket"] = websocket
                    target = data.get("target")
                    sender = get_username_by_ws(websocket)

                    if not target or target not in peers:
                        await websocket.send(json.dumps({"error": "Target user not found"}))
                    else:
                        target_ws = peers[target]["websocket"]

                        # Register the session (both directions)
                        relay_sessions[websocket] = target_ws
                        relay_sessions[target_ws] = websocket

                        response = {
                            "status": "relay_initiated",
                            "type": "relay_initiated",
                            "target": target,
                            "initiator": sender,
                        }

                        await websocket.send(json.dumps(response))      # Tell sender
                        await target_ws.send(json.dumps(response))      # Tell receiver

                        print(f"🔄 Relay started between {sender} and {target}")

            elif msg_type == "relay_receive":
                username = data.get("username")
                if username not in peers:
                    await websocket.send(json.dumps({"error": "username not found"}))
                else:
                    peers[username]["websocket"] = websocket


            elif msg_type == "relay_control" and data.get("action") == "end":
                peer_ws = relay_sessions.pop(websocket, None)
                if peer_ws:
                    await peer_ws.send(json.dumps({
                        "type": "relay_control",
                        "action": "end"
                    }))
                    relay_sessions.pop(peer_ws, None)
                    print("❌ Relay session ended.")

            elif msg_type == "peer_information":
                target = data.get("target")
                if target in peers:
                    info = peers[target]
                    await websocket.send(json.dumps({
                        "type": "peer_info",
                        "pip": info["pip"],
                        "ip": info["ip"],
                        "port": info["port"]
                    }))
                else:
                    await websocket.send(json.dumps({"error": "User not found"}))

            else:
                # Relay all other JSON messages if in a session
                if websocket in relay_sessions:
                    peer_ws = relay_sessions[websocket]
                    await peer_ws.send(message)
                else:
                    await websocket.send(json.dumps({"error": "Unknown or invalid request type"}))

    except websockets.exceptions.ConnectionClosed:
        # print(f"🔌 Client disconnected.")
        # Clean up on disconnect
        # username = get_username_by_ws(websocket)
        # if username:
        #     print(f"Removing user: {username}")
        #     del peers[username]
        print("connection close error")

        peer_ws = relay_sessions.pop(websocket, None)
        if peer_ws:
            relay_sessions.pop(peer_ws, None)
            try:
                await peer_ws.send(json.dumps({
                    "type": "relay_control",
                    "action": "end"
                }))
            except:
                pass


# Server config
server_ip = "0.0.0.0"
server_port = 8765

async def main():
    server = await websockets.serve(
        signalling_handler,
        server_ip,
        server_port,
        max_size=None  # So you can send large files
    )
    print(f"🚀 Signaling server running at {server_ip}:{server_port}")
    await asyncio.Future()  # Run forever

if __name__ == "__main__":
    asyncio.run(main())

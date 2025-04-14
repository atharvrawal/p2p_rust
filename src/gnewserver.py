import asyncio
import socket
import json
import websockets
import traceback # Needed for printing stack traces
import datetime # Added for timestamps in prints

# --- Configuration ---
TCP_HOST = '0.0.0.0'
TCP_PORT = 8765
WS_HOST = '0.0.0.0'
WS_PORT = 9876
BUFFER_SIZE = 8192

# --- Helper Function for Timestamps ---
def print_ts(message):
    """Prints a message with a timestamp."""
    now = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S.%f")[:-3]
    print(f"{now} {message}")

# --- Global State (Use with caution in larger applications) ---
# For TCP server (simple registration, potentially different clients)
tcp_registrations = {}
tcp_userid_counter = 0

# For WebSocket server (P2P signaling and relay)
peers = {}  # {username: {"pip": public_ip, "ip": local_ip, "port": port, "websocket": ws_object}}
relay_sessions = {} # {websocket_sender: websocket_receiver}

# --- TCP Server Logic ---

def handle_tcp_message(message):
    """Processes messages received on the TCP server."""
    global tcp_userid_counter
    global tcp_registrations
    print_ts(f"DEBUG: Handling TCP message: {message}")

    if not isinstance(message, dict):
        print_ts("WARN: TCP: Received non-dictionary message")
        return None # No response for invalid format

    msg_type = message.get("type")
    print_ts(f"DEBUG: TCP message type: {msg_type}")

    if msg_type == "register":
        username = message.get("username")
        if username:
            # Note: This registration is separate from WebSocket peers
            user_id = tcp_userid_counter # Capture current ID before incrementing
            tcp_registrations[user_id] = message
            print_ts(f"INFO: TCP: Registered user ID {user_id}: {username}")
            print_ts(f"DEBUG: TCP Registrations (keys): {list(tcp_registrations.keys())}") # Log keys for brevity
            tcp_userid_counter += 1
            # You might want to send a confirmation back
            # response = json.dumps({"status": "registered", "id": user_id}) + "\n"
            # return response # Or return confirmation if needed
            return None
        else:
            print_ts("WARN: TCP: Registration message missing username")
            return None

    elif msg_type == "get_users":
        status = message.get("status", "N/A") # Get status if provided
        print_ts(f"INFO: TCP: Received get_users request (status: {status})")
        print_ts(f"DEBUG: TCP: Returning users (dict length): {len(tcp_registrations)}")
        # Return the list of TCP registered users
        return json.dumps(tcp_registrations) + "\n"

    elif msg_type == "command":
        cmd = message.get("cmd")
        if cmd:
            print_ts(f"INFO: TCP: Received command: {cmd}")
            # Execute or handle command here if needed
        else:
            print_ts("WARN: TCP: Command message missing 'cmd'")
        return None # No response needed unless command produces output

    else:
        print_ts(f"WARN: TCP: Unknown message type received: {msg_type}")
        return None

async def tcp_server_loop(reader, writer):
    """Handles a single client connection for the TCP server."""
    peer_addr = writer.get_extra_info('peername')
    print_ts(f"INFO: TCP: Connection from {peer_addr}")
    print_ts(f"DEBUG: TCP Reader: {reader}, Writer: {writer}")

    try:
        # Read only one message per connection as per original logic
        data = await reader.read(BUFFER_SIZE)
        if not data:
            print_ts(f"INFO: TCP: No data received from {peer_addr}, closing connection.")
            return # Client disconnected without sending data

        decoded_data = data.decode(errors='ignore') # Decode safely for printing
        print_ts(f"DEBUG: TCP: Received raw data from {peer_addr}: {decoded_data}")

        try:
            message = json.loads(decoded_data)
            print_ts(f"DEBUG: TCP: Parsed message from {peer_addr}: {message}")
            response = handle_tcp_message(message)
            if response:
                print_ts(f"DEBUG: TCP: Sending response to {peer_addr}: {response.strip()}")
                writer.write(response.encode())
                await writer.drain()
                print_ts(f"DEBUG: TCP: Response sent successfully to {peer_addr}")
            else:
                print_ts(f"DEBUG: TCP: No response to send to {peer_addr}")
        except json.JSONDecodeError:
            print_ts(f"ERROR: TCP: Invalid JSON received from {peer_addr}: {decoded_data}")
        except Exception as e:
            print_ts(f"ERROR: TCP: Error handling message from {peer_addr}: {e}")
            traceback.print_exc() # Print stack trace

    except ConnectionResetError:
        print_ts(f"WARN: TCP: Connection reset by {peer_addr}")
    except Exception as e:
        print_ts(f"ERROR: TCP: Error in connection loop with {peer_addr}: {e}")
        traceback.print_exc() # Print stack trace
    finally:
        print_ts(f"INFO: TCP: Closing connection from {peer_addr}")
        try:
            if writer.is_closing():
                print_ts(f"DEBUG: TCP Writer for {peer_addr} already closing.")
            else:
                writer.close()
                await writer.wait_closed()
                print_ts(f"DEBUG: TCP Writer for {peer_addr} closed.")
        except Exception as e:
             print_ts(f"ERROR: TCP: Error closing writer for {peer_addr}: {e}")


async def start_tcp_server(host=TCP_HOST, port=TCP_PORT):
    """Starts the asynchronous TCP server."""
    print_ts(f"DEBUG: Attempting to start TCP server on {host}:{port}")
    try:
        server = await asyncio.start_server(tcp_server_loop, host, port)
        addr = server.sockets[0].getsockname()
        print_ts(f"INFO: 📡 TCP server running on {addr[0]}:{addr[1]}")
        async with server:
            await server.serve_forever()
    except OSError as e:
        print_ts(f"ERROR: ❌ TCP Server failed to start on {host}:{port}: {e}")
    except Exception as e:
        print_ts(f"ERROR: ❌ Unexpected error starting TCP server: {e}")
        traceback.print_exc() # Print stack trace
    print_ts("DEBUG: TCP server task finishing.")


# --- WebSocket Server Logic ---

def get_username_by_ws(ws):
    """Finds the username associated with a websocket connection."""
    # Use ws.id for a unique identifier in prints if available, otherwise use object itself
    ws_repr = getattr(ws, 'id', ws)
    print_ts(f"DEBUG: Searching for username for websocket: {ws_repr}")
    # This is O(n). If performance is critical with many users,
    # consider maintaining a reverse ws -> username map.
    for username, info in peers.items():
        if info.get("websocket") == ws:
            print_ts(f"DEBUG: Found username '{username}' for websocket {ws_repr}")
            return username
    print_ts(f"DEBUG: No username found for websocket {ws_repr}")
    return None

async def cleanup_connection(websocket):
    """Removes user and cleans up relay sessions upon disconnect."""
    ws_repr = getattr(websocket, 'id', websocket) # Get a printable representation
    print_ts(f"DEBUG: Starting cleanup for websocket {ws_repr}")
    username = get_username_by_ws(websocket)
    if username:
        print_ts(f"INFO: 🔌 Client disconnected: {username} (ws: {ws_repr})")
        if username in peers:
             del peers[username]
             print_ts(f"INFO: 🧹 Removed user: {username}. Active users: {list(peers.keys())}")
             print_ts(f"DEBUG: Peers dict keys after removal: {list(peers.keys())}")
        else:
             print_ts(f"WARN: 🧹 Tried to remove non-existent user on disconnect: {username}")
    else:
        print_ts(f"INFO: 🔌 Unregistered client disconnected (ws: {ws_repr}).")

    # Clean up any relay session this user was part of
    # Create a representation of relay_sessions for printing using usernames or ws_repr
    relay_repr = {}
    for k, v in relay_sessions.items():
        k_user = get_username_by_ws(k) or getattr(k, 'id', k)
        v_user = get_username_by_ws(v) or getattr(v, 'id', v)
        relay_repr[k_user] = v_user
    print_ts(f"DEBUG: Checking relay sessions for websocket {ws_repr}. Current sessions: {relay_repr}")

    peer_ws = relay_sessions.pop(websocket, None) # Remove sender -> receiver mapping
    if peer_ws:
        peer_ws_repr = getattr(peer_ws, 'id', peer_ws)
        peer_username = get_username_by_ws(peer_ws)
        print_ts(f"DEBUG: Removed relay session entry: {username or ws_repr} -> {peer_username or peer_ws_repr}")
        # Remove receiver -> sender mapping as well
        relay_sessions.pop(peer_ws, None)
        print_ts(f"DEBUG: Removed reverse relay session entry for {peer_username or peer_ws_repr}")
        print_ts(f"INFO: ❌ Relay session ended due to disconnect involving {username or 'unregistered user'}")
        # Try to notify the other peer that the relay ended
        try:
            if peer_ws.open:
                 end_msg = json.dumps({
                     "type": "relay_control",
                     "action": "end",
                     "reason": f"Peer {username or 'unknown'} disconnected"
                 })
                 print_ts(f"DEBUG: Attempting to send relay end notification to {peer_username or peer_ws_repr}: {end_msg}")
                 await peer_ws.send(end_msg)
                 print_ts(f"INFO: ✉️ Notified peer {peer_username or peer_ws_repr} about relay end.")
            else:
                 print_ts(f"WARN: Peer {peer_username or peer_ws_repr}'s websocket already closed, couldn't notify about relay end.")
        except websockets.exceptions.ConnectionClosed:
             print_ts(f"WARN: Attempted to send relay end notification to {peer_username or peer_ws_repr}, but connection was already closed.")
        except Exception as e:
             print_ts(f"ERROR: Error sending relay end notification to {peer_username or peer_ws_repr}: {e}")
             traceback.print_exc() # Print stack trace
    else:
        print_ts(f"DEBUG: Websocket {ws_repr} was not in an active relay session.")
    print_ts(f"DEBUG: Finished cleanup for websocket {ws_repr}")


async def signalling_handler(websocket):
    """Handles incoming WebSocket connections and messages."""
    ws_repr = getattr(websocket, 'id', websocket) # Get a printable representation
    remote_addr = websocket.remote_address
    print_ts(f"INFO: 🔌 WebSocket client connected from {remote_addr} (ws: {ws_repr})")
    try:
        async for message in websocket:
            sender_username = get_username_by_ws(websocket) # Get username early for context
            sender_repr = sender_username or ws_repr

            # Handle Binary Data (likely for relay)
            if isinstance(message, bytes):
                print_ts(f"DEBUG: Received BINARY data ({len(message)} bytes) from {sender_repr}")
                if websocket in relay_sessions:
                    peer_ws = relay_sessions[websocket]
                    peer_username = get_username_by_ws(peer_ws)
                    peer_repr = peer_username or getattr(peer_ws, 'id', peer_ws)
                    print_ts(f"DEBUG: Relaying BINARY data from {sender_repr} to {peer_repr}")
                    try:
                        await peer_ws.send(message)
                    except websockets.exceptions.ConnectionClosed:
                        print_ts(f"WARN: Attempted to relay binary data to {peer_repr}, but connection is closed.")
                        # Consider cleanup if relay fails consistently
                        # await cleanup_connection(peer_ws)
                    except Exception as e:
                        print_ts(f"ERROR: Failed to relay binary data to {peer_repr}: {e}")

                else:
                    print_ts(f"WARN: ⚠ Received binary message from {sender_repr} but websocket is not in an active relay session.")
                continue # Skip further processing for binary

            # Handle Text Data (JSON expected)
            print_ts(f"DEBUG: Received TEXT data from {sender_repr}: {message}")
            try:
                data = json.loads(message)
                print_ts(f"DEBUG: Parsed JSON from {sender_repr}: {data}")
                msg_type = data.get("type")
                print_ts(f"DEBUG: Message type from {sender_repr}: {msg_type}")

                # --- Message Handling ---
                if msg_type == "register":
                    username = data.get("username")
                    print_ts(f"DEBUG: Processing 'register' request for username: {username}")
                    if username and username not in peers:
                        peers[username] = {
                            "pip": data.get("pip"), # Public IP (if available)
                            "ip": data.get("ip"),   # Local IP (if available)
                            "port": data.get("port"), # Port (if available)
                            "websocket": websocket
                        }
                        response = json.dumps({"status": "registered", "type": "registration_ack"})
                        print_ts(f"DEBUG: Sending registration success to {username}: {response}")
                        await websocket.send(response)
                        print_ts(f"INFO: ✅ Registered WS user: {username}. Active users: {list(peers.keys())}")
                    elif username in peers:
                         error_msg = json.dumps({"error": "Username already taken", "type": "registration_fail"})
                         print_ts(f"WARN: Registration failed for {username}: already exists. Sending: {error_msg}")
                         await websocket.send(error_msg)
                    else:
                         error_msg = json.dumps({"error": "Username missing in registration request", "type": "registration_fail"})
                         print_ts(f"WARN: Registration failed: Username missing. Sending: {error_msg}")
                         await websocket.send(error_msg)

                elif msg_type == "request_peer_list": # Changed from "request_peer" for clarity
                    print_ts(f"DEBUG: Processing 'request_peer_list' from {sender_repr}")
                    usernames = list(peers.keys())
                    # Consider excluding the requester's own username
                    # if sender_username: usernames = [u for u in usernames if u != sender_username]
                    response = json.dumps({"type": "peer_list", "users": usernames})
                    print_ts(f"DEBUG: Sending peer list to {sender_repr}: {response}")
                    await websocket.send(response)
                    print_ts(f"INFO: Sent peer list ({len(usernames)} users) to {sender_repr}")

                elif msg_type == "peer_information":
                    target_username = data.get("target")
                    print_ts(f"DEBUG: Processing 'peer_information' request from {sender_repr} for target: {target_username}")
                    if target_username and target_username in peers:
                        info = peers[target_username]
                        # Don't send the websocket object
                        response_data = {
                            "type": "peer_info",
                            "username": target_username, # Good to include username for context
                            "pip": info.get("pip"),
                            "ip": info.get("ip"),
                            "port": info.get("port")
                        }
                        response = json.dumps(response_data)
                        print_ts(f"DEBUG: Sending peer info for {target_username} to {sender_repr}: {response}")
                        await websocket.send(response)
                        print_ts(f"INFO: Sent info for {target_username} to {sender_repr}")
                    else:
                        error_msg = json.dumps({"error": f"Target user '{target_username}' not found", "type": "peer_info_fail"})
                        print_ts(f"WARN: Peer info request failed for {sender_repr}: User '{target_username}' not found. Sending: {error_msg}")
                        await websocket.send(error_msg)

                elif msg_type == "initiate_relay":
                    target_username = data.get("target")
                    print_ts(f"DEBUG: Processing 'initiate_relay' from {sender_repr} to target: {target_username}")

                    if not sender_username:
                         error_msg = json.dumps({"error": "You must be registered to initiate relay", "type": "relay_fail"})
                         print_ts(f"WARN: Unregistered user {sender_repr} tried to initiate relay. Sending: {error_msg}")
                         await websocket.send(error_msg)
                    elif websocket in relay_sessions:
                         error_msg = json.dumps({"error": "You are already in a relay session", "type": "relay_fail"})
                         print_ts(f"WARN: {sender_repr} tried to initiate relay while already in one. Sending: {error_msg}")
                         await websocket.send(error_msg)
                    elif not target_username or target_username not in peers:
                        error_msg = json.dumps({"error": f"Target user '{target_username}' not found or invalid", "type": "relay_fail"})
                        print_ts(f"WARN: Relay initiation failed for {sender_repr}: Target '{target_username}' not found. Sending: {error_msg}")
                        await websocket.send(error_msg)
                    elif target_username == sender_username:
                         error_msg = json.dumps({"error": "Cannot initiate relay with yourself", "type": "relay_fail"})
                         print_ts(f"WARN: {sender_repr} tried to initiate relay with self. Sending: {error_msg}")
                         await websocket.send(error_msg)
                    else:
                        target_info = peers[target_username]
                        target_ws = target_info["websocket"]
                        target_ws_repr = getattr(target_ws, 'id', target_ws)
                        target_repr = target_username # Use username for target representation

                        if target_ws in relay_sessions:
                             error_msg = json.dumps({"error": f"Target user '{target_repr}' is already in a relay session", "type": "relay_fail"})
                             print_ts(f"WARN: Relay initiation failed for {sender_repr}: Target '{target_repr}' is busy. Sending: {error_msg}")
                             await websocket.send(error_msg)
                        else:
                             print_ts(f"DEBUG: Establishing relay session: {sender_repr} <-> {target_repr}")
                             relay_sessions[websocket] = target_ws
                             relay_sessions[target_ws] = websocket
                             response_initiator = {
                                 "status": "relay_initiated",
                                 "type": "relay_initiated", # Send type for clarity
                                 "peer": target_username,  # Send target to initiator
                                 "initiator": sender_username # Added for clarity
                             }
                             response_target = {
                                 "status": "relay_initiated",
                                 "type": "relay_initiated",
                                 "peer": sender_username, # Send initiator to target
                                 "initiator": sender_username # Added for clarity
                             }
                             print_ts(f"DEBUG: Sending relay_initiated to initiator {sender_repr}: {json.dumps(response_initiator)}")
                             await websocket.send(json.dumps(response_initiator))
                             print_ts(f"DEBUG: Sending relay_initiated to target {target_repr}: {json.dumps(response_target)}")
                             await target_ws.send(json.dumps(response_target))
                             print_ts(f"INFO: 🔄 Relay started between {sender_repr} and {target_repr}")

                elif msg_type == "relay_control" and data.get("action") == "end":
                    print_ts(f"DEBUG: Processing 'relay_control' action 'end' from {sender_repr}")
                    peer_ws = relay_sessions.pop(websocket, None)
                    if peer_ws:
                        peer_ws_repr = getattr(peer_ws, 'id', peer_ws)
                        peer_username = get_username_by_ws(peer_ws)
                        peer_repr = peer_username or peer_ws_repr
                        relay_sessions.pop(peer_ws, None) # Remove reverse mapping
                        print_ts(f"INFO: ❌ Relay session ended by {sender_repr}. Notifying {peer_repr}.")
                        # Notify the other peer
                        try:
                            if peer_ws.open:
                                end_notify_msg = json.dumps({
                                    "type": "relay_control",
                                    "action": "end",
                                    "reason": f"Peer {sender_repr} ended the session."
                                })
                                print_ts(f"DEBUG: Sending relay end notification to {peer_repr}: {end_notify_msg}")
                                await peer_ws.send(end_notify_msg)
                            else:
                                print_ts(f"WARN: Peer {peer_repr}'s websocket already closed when trying to notify relay end.")
                        except websockets.exceptions.ConnectionClosed:
                            print_ts(f"WARN: Attempted to send relay end notification to {peer_repr}, but connection closed.")
                        except Exception as e:
                            print_ts(f"ERROR: Failed to send end notification to {peer_repr}: {e}")
                        # Send confirmation back to the requester
                        ack_msg = json.dumps({"type": "relay_control", "action": "end_ack"})
                        print_ts(f"DEBUG: Sending relay end acknowledgement to {sender_repr}: {ack_msg}")
                        await websocket.send(ack_msg)
                    else:
                        error_msg = json.dumps({"error": "Not currently in a relay session", "type": "relay_control_fail"})
                        print_ts(f"WARN: {sender_repr} tried to end relay, but was not in one. Sending: {error_msg}")
                        await websocket.send(error_msg)

                else:
                    # Handle other JSON messages - relay if in a session
                    print_ts(f"DEBUG: Received message of type '{msg_type}' from {sender_repr}")
                    if websocket in relay_sessions:
                        peer_ws = relay_sessions[websocket]
                        peer_username = get_username_by_ws(peer_ws)
                        peer_repr = peer_username or getattr(peer_ws, 'id', peer_ws)
                        print_ts(f"DEBUG: Relaying JSON message from {sender_repr} to {peer_repr}: {message}")
                        try:
                            # Forward the raw JSON string
                            await peer_ws.send(message)
                        except websockets.exceptions.ConnectionClosed:
                            print_ts(f"WARN: Attempted to relay JSON to {peer_repr}, but connection is closed.")
                            # Consider cleanup if relay fails consistently
                            # await cleanup_connection(peer_ws)
                        except Exception as e:
                            print_ts(f"ERROR: Failed to relay JSON message to {peer_repr}: {e}")
                    else:
                        # Unknown message type and not in relay session
                        error_msg = json.dumps({"error": "Unknown or invalid request type", "type": "error"})
                        print_ts(f"WARN: WS: Unknown/invalid message type '{msg_type}' from {sender_repr}. Not in relay. Sending: {error_msg}")
                        await websocket.send(error_msg)

            except json.JSONDecodeError:
                print_ts(f"ERROR: WS: Invalid JSON received from {sender_repr}@{remote_addr}: {message}")
                try:
                    await websocket.send(json.dumps({"error": "Invalid JSON format", "type": "error"}))
                except: pass # Ignore if sending error fails
            except Exception as e:
                print_ts(f"ERROR: WS: Error processing message from {sender_repr}: {e}")
                traceback.print_exc() # Print stack trace
                try:
                     # Attempt to send a generic error back to the client
                     await websocket.send(json.dumps({"error": "Server processing error", "type": "error"}))
                except:
                     pass # Ignore if sending error fails

    except websockets.exceptions.ConnectionClosedOK:
        print_ts(f"INFO: 🔌 WebSocket connection closed normally from {remote_addr} (ws: {ws_repr})")
    except websockets.exceptions.ConnectionClosedError as e:
        print_ts(f"WARN: 🔌 WebSocket connection closed with error from {remote_addr} (ws: {ws_repr}): {e}")
    except Exception as e:
        print_ts(f"ERROR: WS: Unexpected error in handler for {remote_addr} (ws: {ws_repr}): {e}")
        traceback.print_exc() # Print stack trace
    finally:
        # Ensure cleanup happens regardless of how the connection closes
        await cleanup_connection(websocket)


async def start_websocket_server(host=WS_HOST, port=WS_PORT):
    """Starts the asynchronous WebSocket server."""
    print_ts(f"DEBUG: Attempting to start WebSocket server on {host}:{port}")
    try:
        # Wrap handler to catch exceptions during server setup itself if needed
        async def wrapped_handler(websocket):
            try:
                 await signalling_handler(websocket)
            except Exception as e:
                 ws_repr = getattr(websocket, 'id', websocket)
                 print_ts(f"CRITICAL: Unhandled exception in signalling_handler for {ws_repr}: {e}")
                 traceback.print_exc()
                 # Try to close the connection gracefully if possible
                 try:
                     await websocket.close(code=1011, reason="Internal server error")
                 except:
                     pass # Ignore errors during close

        server = await websockets.serve(wrapped_handler, host, port)
        # Use server.sockets to get actual bound address if host was 0.0.0.0
        actual_host, actual_port = server.sockets[0].getsockname()[:2]
        print_ts(f"INFO: 📡 WebSocket server running on ws://{actual_host}:{actual_port}")
        await server.wait_closed() # Keep server running until stopped
    except OSError as e:
        print_ts(f"ERROR: ❌ WebSocket Server failed to start on {host}:{port}: {e}")
    except Exception as e:
        print_ts(f"ERROR: ❌ Unexpected error starting WebSocket server: {e}")
        traceback.print_exc() # Print stack trace
    print_ts("DEBUG: WebSocket server task finishing.")


async def main():
    """Runs both TCP and WebSocket servers concurrently."""
    print_ts("INFO: 🚀 Starting servers...")
    # Create tasks for each server
    websocket_server_task = asyncio.create_task(start_websocket_server())
    tcp_server_task = asyncio.create_task(start_tcp_server())

    # Wait for either task to complete (which they shouldn't unless there's an error or shutdown)
    done, pending = await asyncio.wait(
        [websocket_server_task, tcp_server_task],
        return_when=asyncio.FIRST_COMPLETED,
    )

    # If a task finishes, log it and cancel the others
    for task in done:
        try:
             # Access result to raise exception if task failed
             task.result()
             print_ts("WARN: A server task finished unexpectedly without error.")
        except Exception as e:
             print_ts(f"ERROR: Server task finished unexpectedly with error: {e}")
             traceback.print_exc() # Print stack trace

    print_ts("WARN: A server task has finished. Shutting down remaining tasks...")
    for task in pending:
        task.cancel()
        try:
            # Wait for task cancellation to complete
            await task
        except asyncio.CancelledError:
             print_ts("INFO: Server task cancelled.")
        except Exception as e:
             # Log errors during cancellation itself
             print_ts(f"ERROR: Error during server task cancellation: {e}")
             traceback.print_exc()

    print_ts("INFO: 🛑 Servers stopped.")


# Corrected main execution block
if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print_ts("\nINFO: 🚦 Received exit signal (Ctrl+C). Shutting down...")
        # asyncio implicitly handles cancelling tasks on KeyboardInterrupt in run()
    except Exception as e:
         print_ts(f"CRITICAL: 💥 Unhandled exception in main loop: {e}")
         traceback.print_exc() # Print stack trace
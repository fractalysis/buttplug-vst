#!/usr/bin/env python

import asyncio
import websockets
import signal
from signal import SIGINT, SIGTERM

async def handler(websocket):
    while True:
        try:
            message = await websocket.recv()
            print(message)
        except websockets.ConnectionClosedOK:
            print("Connection closed")
            break
        except websockets.ConnectionClosedError:
            print("Connection closed forcibly")
            break
        except Exception as e:
            print(e)
            break


async def main():
    async with websockets.serve(handler, "", 12345):
        print("Initialized.")
        await asyncio.Future()  # run forever


if __name__ == "__main__":
    asyncio.run(main())
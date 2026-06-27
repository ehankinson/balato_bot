import pyautogui
import time

try:
    while True:
        x, y = pyautogui.position()
        print(f"x={x}, y={y}")
        time.sleep(0.25)
except KeyboardInterrupt:
    print("done")

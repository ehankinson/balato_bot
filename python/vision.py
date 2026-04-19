from ultralytics import YOLO
import cv2

# Load pretrained model
model = YOLO("yolov8n.pt")  # small + fast

# Load your screenshot
img = cv2.imread("9_cards.png")

# Run detection
results = model(img)

# Draw results
annotated = results[0].plot()

cv2.imwrite("output.png", annotated)

print("Saved output.png")
from reportlab.lib.colors import HexColor
from reportlab.lib.pagesizes import A4
from reportlab.pdfgen import canvas


OUTPUT = "tmp/pdfs/restless-effect-test-attachment.pdf"

page = canvas.Canvas(OUTPUT, pagesize=A4)
width, height = A4
page.setFillColor(HexColor("#111827"))
page.rect(0, 0, width, height, stroke=0, fill=1)
page.setFillColor(HexColor("#F9FAFB"))
page.setFont("Helvetica-Bold", 32)
page.drawString(54, height - 92, "RESTLESS EFFECT TEST")
page.setFillColor(HexColor("#F59E0B"))
page.setFont("Helvetica-Bold", 18)
page.drawString(54, height - 132, "TEST ONLY - NO LIVE SEND")
page.setFillColor(HexColor("#D1D5DB"))
page.setFont("Helvetica", 12)
page.drawString(54, height - 182, "Attachment fixture for the generic governed CLI dry-run.")
page.drawString(54, height - 204, "It contains no customer, tutoring centre, or Aris sales content.")
page.setStrokeColor(HexColor("#374151"))
page.line(54, height - 230, width - 54, height - 230)
page.setFont("Helvetica", 10)
page.setFillColor(HexColor("#9CA3AF"))
page.drawString(54, 52, "Generated for Sprint 05 T8 acceptance evidence")
page.save()

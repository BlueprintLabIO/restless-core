import fitz


document = fitz.open("tmp/pdfs/restless-effect-test-attachment.pdf")
assert document.page_count == 1
page = document[0]
page.get_pixmap(matrix=fitz.Matrix(1.6, 1.6), alpha=False).save(
    "tmp/pdfs/restless-effect-test-attachment.png"
)
text = page.get_text()
assert "TEST ONLY - NO LIVE SEND" in text
assert "no customer, tutoring centre, or Aris sales content" in text

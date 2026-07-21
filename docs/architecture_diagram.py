import pdfkit
import os

# Path to the HTML file
html_file = '/home/yaya/le-coq/docs/ARCHITECTURE_DIAGRAM.html'

# Path to the output PDF file
pdf_file = '/home/yaya/le-coq/docs/ARCHITECTURE_DIAGRAM.pdf'

# Check if the HTML file exists
if not os.path.exists(html_file):
    print(f"Error: HTML file not found at {html_file}")
else:
    # Convert HTML to PDF
    try:
        pdfkit.from_file(html_file, pdf_file)
        print(f"PDF successfully generated at {pdf_file}")
    except Exception as e:
        print(f"Error generating PDF: {e}")
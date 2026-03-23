#!/usr/bin/env python3
"""Generate one test PDF per practice area for Justice AI eval cases.

Run: python3 generate_practice_area_pdfs.py

Creates 8 PDFs in the current directory:
  1. criminal_charge.pdf        — Criminal complaint (Criminal Law)
  2. divorce_decree.pdf         — Final decree of divorce (Family / Domestic)
  3. corporate_merger.pdf       — Merger agreement (Corporate / Contract)
  4. immigration_petition.pdf   — I-140 petition (Immigration)
  5. personal_injury_complaint.pdf — PI complaint (Personal Injury)
  6. commercial_lease.pdf       — Commercial lease (Real Estate / Property)
  7. employment_agreement.pdf   — Employment agreement (Employment / Labor)
  8. compliance_report.pdf      — Compliance audit report (Regulatory / Compliance)
"""

from reportlab.lib.pagesizes import letter
from reportlab.pdfgen import canvas
from reportlab.lib.units import inch

W, H = letter  # 612 x 792


def draw_lines(c, x, y, lines, font="Helvetica", size=11, leading=16, blank_skip=14):
    """Draw a list of text lines, skipping blank lines with extra spacing."""
    c.setFont(font, size)
    for line in lines:
        if line == "":
            y -= blank_skip
            continue
        c.drawString(x, y, line)
        y -= leading
    return y


# ── 1. Criminal Charge ──────────────────────────────────────────────────────

def make_criminal_charge():
    c = canvas.Canvas("criminal_charge.pdf", pagesize=letter)

    c.setFont("Helvetica-Bold", 14)
    c.drawString(1*inch, H - 1*inch, "STATE OF GEORGIA")
    c.drawString(1*inch, H - 1.3*inch, "FULTON COUNTY STATE COURT")
    c.setFont("Helvetica-Bold", 16)
    c.drawString(1*inch, H - 1.8*inch, "CRIMINAL COMPLAINT")

    y = H - 2.4*inch
    lines = [
        "Case Number: 2025-CR-00847",
        "",
        "STATE OF GEORGIA vs. MARCUS JOHNSON",
        "",
        "The undersigned officer, having been duly sworn, states the following:",
        "",
        "DEFENDANT INFORMATION:",
        "  Name: Marcus Johnson",
        "  Date of Birth: 03/15/1992",
        "  Address: 1190 Boulevard SE, Atlanta, GA 30312",
        "",
        "CHARGE: Aggravated Assault, in violation of O.C.G.A. Section 16-5-21",
        "Maximum Sentence: 20 years imprisonment",
        "",
        "FACTS OF THE CASE:",
        "On March 3, 2025, at approximately 11:45 PM, the defendant Marcus Johnson",
        "did commit an aggravated assault upon the victim, Sarah Chen, at 422 Peachtree",
        "St NE, Atlanta, GA 30308. The defendant struck the victim with a baseball bat,",
        "causing serious bodily injury including a fractured left forearm and lacerations",
        "to the head requiring 14 stitches. The victim was transported to Grady Memorial",
        "Hospital where she remained for 3 days.",
        "",
        "Witnesses at the scene identified the defendant. Surveillance footage from the",
        "adjacent business at 420 Peachtree St NE corroborates the witness accounts.",
        "The weapon, a 34-inch aluminum baseball bat, was recovered at the scene.",
        "",
        "ARRESTING OFFICER: Detective James Rivera, Badge #4472",
        "Atlanta Police Department, Zone 5",
        "Date of Arrest: March 4, 2025",
        "",
        "BOND: $25,000 cash or surety",
        "",
        "ARRAIGNMENT: March 17, 2025, Courtroom 4B, Fulton County State Court",
        "",
        "Sworn to and subscribed before me this 4th day of March, 2025.",
        "",
        "____________________________",
        "Det. James Rivera, Badge #4472",
    ]
    draw_lines(c, 1*inch, y, lines, size=10, leading=14, blank_skip=10)

    c.save()
    print("  Created criminal_charge.pdf")


# ── 2. Divorce Decree ───────────────────────────────────────────────────────

def make_divorce_decree():
    c = canvas.Canvas("divorce_decree.pdf", pagesize=letter)

    c.setFont("Helvetica-Bold", 14)
    c.drawString(1*inch, H - 1*inch, "IN THE SUPERIOR COURT OF FULTON COUNTY")
    c.drawString(1*inch, H - 1.3*inch, "STATE OF GEORGIA")
    c.setFont("Helvetica-Bold", 16)
    c.drawString(1*inch, H - 1.8*inch, "FINAL DECREE OF DIVORCE")

    y = H - 2.4*inch
    lines = [
        "Case Number: 2025-CV-12034",
        "",
        "Petitioner: Amanda R. Foster",
        "Respondent: Thomas J. Foster",
        "",
        "This matter came before the Court on the Petition for Divorce filed",
        "February 20, 2025. The parties were married on June 14, 2015, in",
        "Savannah, Georgia, and separated on January 8, 2024.",
        "",
        "IT IS HEREBY ORDERED, ADJUDGED, AND DECREED:",
        "",
        "1. DISSOLUTION: The marriage between Amanda R. Foster and Thomas J.",
        "   Foster is dissolved effective upon entry of this decree.",
        "",
        "2. CHILD CUSTODY: The parties have one minor child, Emily Foster,",
        "   born September 22, 2018 (age 6). The parties shall share joint legal",
        "   custody. Petitioner is awarded primary physical custody. Respondent",
        "   shall have parenting time every other weekend and Wednesday evenings.",
        "",
        "3. CHILD SUPPORT: Respondent shall pay $1,450 per month in child",
        "   support, due on the 1st of each month, until the child reaches age 18",
        "   or graduates high school, whichever occurs later.",
        "",
        "4. ALIMONY: Respondent shall pay $2,200 per month in rehabilitative",
        "   alimony for a period of 36 months, beginning April 1, 2025.",
        "",
        "5. MARITAL HOME: The marital residence located at 1847 Roswell Rd,",
        "   Atlanta, GA 30309 is awarded to Petitioner. Respondent shall",
        "   execute a quitclaim deed within 30 days of this order.",
        "",
        "6. RETIREMENT ACCOUNTS: The marital portion of Respondent's 401(k)",
        "   account shall be divided 55% to Petitioner and 45% to Respondent",
        "   via Qualified Domestic Relations Order (QDRO).",
        "",
    ]
    y = draw_lines(c, 1*inch, y, lines, size=10, leading=14, blank_skip=10)

    # Page 2
    c.showPage()
    c.setFont("Helvetica-Bold", 11)
    c.drawString(1*inch, H - 1*inch, "Case No. 2025-CV-12034 — Final Decree (cont.)")

    y = H - 1.6*inch
    lines2 = [
        "7. VEHICLES: Each party retains the vehicle currently in their possession.",
        "   Petitioner: 2022 Toyota RAV4. Respondent: 2020 Ford F-150.",
        "",
        "8. DEBTS: Each party shall be responsible for debts incurred in their",
        "   individual name after the date of separation, January 8, 2024.",
        "",
        "SO ORDERED this 15th day of March, 2025.",
        "",
        "____________________________",
        "Hon. Patricia N. Blackwell",
        "Superior Court Judge, Fulton County",
    ]
    draw_lines(c, 1*inch, y, lines2, size=10, leading=14, blank_skip=10)

    c.save()
    print("  Created divorce_decree.pdf")


# ── 3. Corporate Merger Agreement ───────────────────────────────────────────

def make_corporate_merger():
    c = canvas.Canvas("corporate_merger.pdf", pagesize=letter)

    c.setFont("Helvetica-Bold", 16)
    c.drawString(1*inch, H - 1*inch, "AGREEMENT AND PLAN OF MERGER")

    y = H - 1.6*inch
    lines = [
        "This Agreement and Plan of Merger (the \"Agreement\") is entered into as of",
        "March 1, 2025, by and between:",
        "",
        "ACQUIRING COMPANY: Nexus Technologies Inc., a Delaware corporation,",
        "  with principal offices at 800 Innovation Drive, Wilmington, DE 19801",
        "  (\"Acquirer\")",
        "",
        "TARGET COMPANY: CloudBridge Solutions LLC, a California limited liability",
        "  company, with principal offices at 1500 Pacific Coast Hwy, Santa Monica,",
        "  CA 90401 (\"Target\")",
        "",
        "ARTICLE I — THE MERGER",
        "",
        "1.1 Purchase Price. The total purchase price shall be Forty-Seven Million",
        "    Five Hundred Thousand Dollars ($47,500,000), payable in cash at closing.",
        "",
        "1.2 Closing Date. The closing shall occur on April 30, 2025, subject to",
        "    satisfaction of all conditions precedent.",
        "",
        "1.3 Escrow. $4,750,000 (10% of purchase price) shall be deposited with",
        "    First National Trust as escrow agent, to be held for 18 months following",
        "    closing as security for indemnification obligations.",
        "",
        "ARTICLE II — DUE DILIGENCE",
        "",
        "2.1 Due Diligence Period. The Acquirer shall have 45 days from the date of",
        "    this Agreement (through April 14, 2025) to complete due diligence.",
        "",
        "ARTICLE III — KEY EMPLOYEES AND RETENTION",
        "",
        "3.1 Key Employees. The following individuals are deemed Key Employees:",
        "    CEO David Park and CTO Lisa Yamamoto.",
        "",
        "3.2 Retention Bonus Pool. Acquirer shall establish an employee retention",
        "    bonus pool of $3,200,000, distributed over 24 months post-closing.",
        "",
        "ARTICLE IV — RESTRICTIVE COVENANTS",
        "",
        "4.1 Non-Compete. Sellers and Key Employees agree to a non-compete period",
        "    of 24 months within a 150-mile radius of any Target office location.",
        "",
    ]
    y = draw_lines(c, 1*inch, y, lines, size=10, leading=14, blank_skip=10)

    # Page 2
    c.showPage()
    c.setFont("Helvetica-Bold", 11)
    c.drawString(1*inch, H - 1*inch, "Agreement and Plan of Merger (cont.)")

    y = H - 1.6*inch
    lines2 = [
        "ARTICLE V — TERMINATION",
        "",
        "5.1 Break-Up Fee. If either party terminates this Agreement other than",
        "    for cause, the terminating party shall pay a break-up fee of",
        "    $2,375,000 (5% of purchase price) to the non-terminating party.",
        "",
        "ARTICLE VI — REPRESENTATIONS AND WARRANTIES",
        "",
        "6.1 Survival. All representations and warranties contained in this",
        "    Agreement shall survive the closing for a period of 24 months.",
        "",
        "ARTICLE VII — GOVERNING LAW",
        "",
        "7.1 This Agreement shall be governed by and construed in accordance with",
        "    the laws of the State of Delaware, without regard to conflict of laws.",
        "",
        "IN WITNESS WHEREOF, the parties have executed this Agreement as of the",
        "date first written above.",
        "",
        "NEXUS TECHNOLOGIES INC.",
        "By: ____________________________",
        "Name: Jonathan Mercer, President",
        "",
        "CLOUDBRIDGE SOLUTIONS LLC",
        "By: ____________________________",
        "Name: David Park, CEO",
    ]
    draw_lines(c, 1*inch, y, lines2, size=10, leading=14, blank_skip=10)

    c.save()
    print("  Created corporate_merger.pdf")


# ── 4. Immigration Petition ─────────────────────────────────────────────────

def make_immigration_petition():
    c = canvas.Canvas("immigration_petition.pdf", pagesize=letter)

    c.setFont("Helvetica-Bold", 14)
    c.drawString(1*inch, H - 1*inch, "U.S. CITIZENSHIP AND IMMIGRATION SERVICES")
    c.setFont("Helvetica-Bold", 16)
    c.drawString(1*inch, H - 1.5*inch, "FORM I-140 — IMMIGRANT PETITION")
    c.setFont("Helvetica-Bold", 12)
    c.drawString(1*inch, H - 1.85*inch, "FOR ALIEN WORKERS")

    y = H - 2.5*inch
    lines = [
        "PART 1 — PETITIONER (EMPLOYER) INFORMATION",
        "",
        "  Company Name: BioGenesis Research Corp.",
        "  Address: 2200 Research Blvd, San Diego, CA 92121",
        "  Federal EIN: 85-4273619",
        "  Year Established: 2011",
        "  Number of Employees: 340",
        "  Gross Annual Revenue: $78,000,000",
        "",
        "PART 2 — CLASSIFICATION REQUESTED",
        "",
        "  Preference Category: EB-2 (Advanced Degree Professional)",
        "  Priority Date: November 15, 2024",
        "  PERM Labor Certification Case Number: A-18945-73621",
        "",
        "PART 3 — BENEFICIARY INFORMATION",
        "",
        "  Name: Dr. Anika Sharma",
        "  Date of Birth: 08/12/1988",
        "  Country of Birth: India",
        "  Country of Nationality: India",
        "  Current Immigration Status: H-1B (valid through 09/30/2026)",
        "  A-Number: A-217-856-443",
        "",
        "PART 4 — JOB OFFER DETAILS",
        "",
        "  Position Title: Senior Research Scientist",
        "  SOC Code: 19-1042",
        "  Offered Salary: $142,000 per year",
        "  Prevailing Wage: $128,500 per year (Level III, San Diego-Chula Vista MSA)",
        "  Full-Time: Yes (40 hours/week)",
        "  Work Location: 2200 Research Blvd, San Diego, CA 92121",
        "",
        "PART 5 — BENEFICIARY QUALIFICATIONS",
        "",
        "  Highest Degree: Ph.D. in Molecular Biology",
        "  Institution: Massachusetts Institute of Technology (MIT)",
        "  Year Conferred: 2016",
        "  Undergraduate: B.Sc. Biotechnology, Indian Institute of Technology Delhi, 2010",
        "",
        "  Publications: 23 peer-reviewed articles in journals including Nature",
        "  Biotechnology, Cell Reports, and Journal of Molecular Biology.",
        "  Citations: 847 total (Google Scholar, as of October 2024).",
        "  Patents: 2 U.S. patents (US11,234,567 and US11,345,678).",
        "",
        "  The beneficiary's work in CRISPR-based gene therapy has been recognized",
        "  through invitations to speak at 6 international conferences and service",
        "  as a peer reviewer for 3 leading journals.",
        "",
        "Petitioner Signature: ____________________________",
        "Name: Dr. Henry Lau, Director of Human Resources",
        "Date: November 12, 2024",
    ]
    draw_lines(c, 1*inch, y, lines, size=10, leading=13, blank_skip=9)

    c.save()
    print("  Created immigration_petition.pdf")


# ── 5. Personal Injury Complaint ────────────────────────────────────────────

def make_personal_injury_complaint():
    c = canvas.Canvas("personal_injury_complaint.pdf", pagesize=letter)

    c.setFont("Helvetica-Bold", 14)
    c.drawString(1*inch, H - 1*inch, "SUPERIOR COURT OF CALIFORNIA")
    c.drawString(1*inch, H - 1.3*inch, "COUNTY OF SACRAMENTO")
    c.setFont("Helvetica-Bold", 16)
    c.drawString(1*inch, H - 1.8*inch, "COMPLAINT FOR PERSONAL INJURIES")

    y = H - 2.4*inch
    lines = [
        "Case Number: 2025-PI-03291",
        "",
        "MICHAEL TORRES, Plaintiff,",
        "  vs.",
        "SWIFT DELIVERY SERVICES INC., Defendant.",
        "",
        "Plaintiff Michael Torres, age 34, by and through his attorneys, hereby",
        "complains against Defendant Swift Delivery Services Inc. as follows:",
        "",
        "FACTS:",
        "",
        "1. On February 12, 2025, at approximately 3:20 PM, Plaintiff was operating",
        "   his 2021 Honda Civic southbound on Main Street in Sacramento, California.",
        "",
        "2. At the intersection of Main Street and 5th Avenue, Defendant's employee",
        "   Kevin Brooks, operating a Swift Delivery Services commercial van",
        "   (CA License Plate: 8ABC123), rear-ended Plaintiff's vehicle at an",
        "   estimated speed of 35 mph while Plaintiff was stopped at a red light.",
        "",
        "3. Kevin Brooks was acting within the course and scope of his employment",
        "   with Swift Delivery Services Inc. at the time of the collision.",
        "",
        "INJURIES:",
        "",
        "4. As a direct result of the collision, Plaintiff sustained the following",
        "   injuries, as diagnosed by Dr. Elena Vasquez at Sacramento Medical Center:",
        "   a) Herniated disc at L4-L5",
        "   b) Cervical strain (whiplash)",
        "   c) Grade 2 concussion",
        "",
        "5. Plaintiff underwent an MRI on February 14, 2025, confirming the L4-L5",
        "   herniation. Plaintiff has attended 24 physical therapy sessions to date",
        "   and may require surgical intervention (lumbar microdiscectomy).",
        "",
        "DAMAGES:",
        "",
        "6. Medical expenses incurred to date: $87,400.00",
        "   (Emergency room, MRI, orthopedic consultations, physical therapy)",
        "",
        "7. Lost wages: $23,600.00 (14 weeks of missed work at $1,685/week)",
        "",
        "8. Property damage to 2021 Honda Civic: $12,800.00",
        "",
        "9. Pain and suffering: $150,000.00",
        "",
        "TOTAL DAMAGES SOUGHT: $273,800.00",
    ]
    y = draw_lines(c, 1*inch, y, lines, size=10, leading=13, blank_skip=9)

    # Page 2
    c.showPage()
    c.setFont("Helvetica-Bold", 11)
    c.drawString(1*inch, H - 1*inch, "Torres v. Swift Delivery Services Inc. (cont.)")

    y = H - 1.6*inch
    lines2 = [
        "CAUSES OF ACTION:",
        "",
        "FIRST CAUSE OF ACTION — Negligence",
        "10. Defendant Kevin Brooks owed a duty of care to operate his vehicle",
        "    safely. He breached this duty by failing to stop and rear-ending",
        "    Plaintiff's vehicle. This breach directly caused Plaintiff's injuries.",
        "",
        "SECOND CAUSE OF ACTION — Respondeat Superior",
        "11. Defendant Swift Delivery Services Inc. is vicariously liable for the",
        "    negligent acts of its employee Kevin Brooks committed within the scope",
        "    of his employment.",
        "",
        "PRAYER FOR RELIEF:",
        "Plaintiff requests judgment against Defendant for:",
        "  (a) Compensatory damages in the amount of $273,800.00;",
        "  (b) Costs of suit;",
        "  (c) Such other and further relief as the Court deems just.",
        "",
        "Dated: March 5, 2025",
        "",
        "____________________________",
        "Attorney for Plaintiff",
        "Reyes & Associates LLP",
        "450 Capitol Mall, Suite 1200",
        "Sacramento, CA 95814",
    ]
    draw_lines(c, 1*inch, y, lines2, size=10, leading=14, blank_skip=10)

    c.save()
    print("  Created personal_injury_complaint.pdf")


# ── 6. Commercial Lease ─────────────────────────────────────────────────────

def make_commercial_lease():
    c = canvas.Canvas("commercial_lease.pdf", pagesize=letter)

    c.setFont("Helvetica-Bold", 16)
    c.drawString(1*inch, H - 1*inch, "COMMERCIAL LEASE AGREEMENT")

    y = H - 1.6*inch
    lines = [
        "This Commercial Lease Agreement (\"Lease\") is entered into as of April 1, 2025,",
        "by and between:",
        "",
        "LANDLORD: Metropolitan Property Group LLC",
        "  Address: 200 South Broad Street, Suite 800, Philadelphia, PA 19102",
        "",
        "TENANT: Sunrise Bakery Inc.",
        "  Address: 3500 Market Street, Suite 102, Philadelphia, PA 19104",
        "",
        "1. PREMISES: The Landlord leases to Tenant the property located at",
        "   3500 Market Street, Suite 102, Philadelphia, PA 19104, consisting of",
        "   approximately 1,800 square feet of ground-floor retail space.",
        "",
        "2. TERM: Five (5) years, commencing May 1, 2025 and expiring April 30, 2030.",
        "",
        "3. BASE RENT:",
        "   Year 1: $4,200 per month ($50,400 annually)",
        "   Year 2: $4,326 per month (3% escalation)",
        "   Year 3: $4,456 per month",
        "   Year 4: $4,590 per month",
        "   Year 5: $4,727 per month",
        "   Rent is due on the 1st of each month. Late fee: $150 after the 5th.",
        "",
        "4. SECURITY DEPOSIT: $12,600 (equivalent to 3 months of Year 1 rent),",
        "   due upon execution of this Lease.",
        "",
        "5. COMMON AREA MAINTENANCE (CAM): Tenant shall pay $850 per month",
        "   for common area maintenance, trash removal, and shared utilities.",
        "",
        "6. PERMITTED USE: The premises shall be used solely as a bakery and cafe.",
        "   No other use is permitted without Landlord's written consent.",
        "",
        "7. BUILD-OUT ALLOWANCE: Landlord shall provide a tenant improvement",
        "   allowance of $35,000 for initial build-out, payable upon completion",
        "   and Landlord approval of improvements.",
        "",
        "8. PERSONAL GUARANTEE: Jessica Huang, as owner of Sunrise Bakery Inc.,",
        "   personally guarantees all obligations under this Lease.",
        "",
    ]
    y = draw_lines(c, 1*inch, y, lines, size=10, leading=13, blank_skip=9)

    # Page 2
    c.showPage()
    c.setFont("Helvetica-Bold", 11)
    c.drawString(1*inch, H - 1*inch, "Commercial Lease Agreement (cont.)")

    y = H - 1.6*inch
    lines2 = [
        "9. RENEWAL OPTIONS: Tenant shall have the option to renew this Lease",
        "   for two (2) additional terms of three (3) years each, upon written",
        "   notice to Landlord at least 180 days prior to expiration.",
        "",
        "10. EARLY TERMINATION: Tenant may terminate this Lease after the end",
        "    of Year 3 (after April 30, 2028) by providing six (6) months",
        "    written notice and paying an early termination penalty equal to",
        "    three (3) months of then-current base rent.",
        "",
        "11. INSURANCE: Tenant shall maintain commercial general liability",
        "    insurance with minimum coverage of $1,000,000 per occurrence and",
        "    $2,000,000 aggregate, naming Landlord as additional insured.",
        "",
        "12. MAINTENANCE: Tenant is responsible for interior maintenance and",
        "    repairs. Landlord is responsible for structural elements, roof,",
        "    and building systems (HVAC, plumbing, electrical).",
        "",
        "13. GOVERNING LAW: This Lease shall be governed by the laws of the",
        "    Commonwealth of Pennsylvania.",
        "",
        "IN WITNESS WHEREOF, the parties have executed this Lease as of the",
        "date first written above.",
        "",
        "METROPOLITAN PROPERTY GROUP LLC",
        "By: ____________________________",
        "Name: Richard Donovan, Managing Partner",
        "",
        "SUNRISE BAKERY INC.",
        "By: ____________________________",
        "Name: Jessica Huang, Owner",
    ]
    draw_lines(c, 1*inch, y, lines2, size=10, leading=14, blank_skip=10)

    c.save()
    print("  Created commercial_lease.pdf")


# ── 7. Employment Agreement ─────────────────────────────────────────────────

def make_employment_agreement():
    c = canvas.Canvas("employment_agreement.pdf", pagesize=letter)

    c.setFont("Helvetica-Bold", 16)
    c.drawString(1*inch, H - 1*inch, "EMPLOYMENT AGREEMENT")

    y = H - 1.6*inch
    lines = [
        "This Employment Agreement (\"Agreement\") is entered into as of March 1, 2025,",
        "by and between:",
        "",
        "EMPLOYER: Vertex Analytics Corp., a New York corporation",
        "  Address: 350 Fifth Avenue, Suite 4200, New York, NY 10118",
        "",
        "EMPLOYEE: Rachel Kim",
        "  Address: 88 Greenwich Street, Apt 14C, New York, NY 10006",
        "",
        "1. POSITION AND DUTIES: Employee is hired as Vice President of Engineering,",
        "   reporting to CTO Daniel Okafor. Start date: March 15, 2025.",
        "",
        "2. COMPENSATION:",
        "   a) Base Salary: $225,000 per year, paid semi-monthly.",
        "   b) Signing Bonus: $30,000, paid within 30 days of start date.",
        "      The signing bonus is repayable in full if Employee voluntarily",
        "      departs within 12 months of start date.",
        "   c) Annual Bonus: Target bonus of 25% of base salary ($56,250),",
        "      based on individual and company performance metrics.",
        "",
        "3. EQUITY COMPENSATION:",
        "   a) RSU Grant: 15,000 shares of Vertex Analytics Corp. common stock.",
        "   b) Vesting: 4-year vesting schedule with a 25% cliff at 1 year",
        "      (3,750 shares vest on March 15, 2026), then monthly vesting of",
        "      the remaining 11,250 shares over the subsequent 36 months.",
        "",
        "4. BENEFITS: Employee is eligible for standard company benefits including",
        "   health insurance, dental, vision, 401(k) with 4% match, and 20 days",
        "   paid time off per year.",
        "",
        "5. TERMINATION:",
        "   a) Without Cause: If terminated by Employer without cause, Employee",
        "      shall receive severance of 6 months base salary ($112,500) plus",
        "      prorated annual bonus, subject to execution of a release agreement.",
        "   b) For Cause: No severance or bonus upon termination for cause.",
        "   c) Voluntary Resignation: 30 days written notice required.",
        "",
    ]
    y = draw_lines(c, 1*inch, y, lines, size=10, leading=13, blank_skip=9)

    # Page 2
    c.showPage()
    c.setFont("Helvetica-Bold", 11)
    c.drawString(1*inch, H - 1*inch, "Employment Agreement — Rachel Kim (cont.)")

    y = H - 1.6*inch
    lines2 = [
        "6. RESTRICTIVE COVENANTS:",
        "   a) Non-Compete: For 12 months following termination, Employee shall",
        "      not work for a competing company in the technology sector.",
        "   b) Non-Solicitation: For 18 months following termination, Employee",
        "      shall not solicit any employees or clients of the Employer.",
        "   c) Confidentiality: Employee shall not disclose any proprietary or",
        "      confidential information of the Employer, in perpetuity.",
        "",
        "7. INVENTION ASSIGNMENT: All inventions, discoveries, and work product",
        "   created by Employee during the term of employment and related to the",
        "   Employer's business shall be the sole property of the Employer.",
        "",
        "8. DISPUTE RESOLUTION: Any disputes arising under this Agreement shall",
        "   be resolved by binding arbitration in New York, NY under AAA rules.",
        "",
        "9. GOVERNING LAW: This Agreement shall be governed by the laws of the",
        "   State of New York.",
        "",
        "10. ENTIRE AGREEMENT: This Agreement constitutes the entire agreement",
        "    between the parties and supersedes all prior negotiations and",
        "    agreements, whether written or oral.",
        "",
        "IN WITNESS WHEREOF, the parties have executed this Agreement as of the",
        "date first written above.",
        "",
        "VERTEX ANALYTICS CORP.",
        "By: ____________________________",
        "Name: Daniel Okafor, Chief Technology Officer",
        "Date: March 1, 2025",
        "",
        "EMPLOYEE:",
        "____________________________",
        "Rachel Kim",
        "Date: March 1, 2025",
    ]
    draw_lines(c, 1*inch, y, lines2, size=10, leading=14, blank_skip=10)

    c.save()
    print("  Created employment_agreement.pdf")


# ── 8. Compliance Audit Report ──────────────────────────────────────────────

def make_compliance_report():
    c = canvas.Canvas("compliance_report.pdf", pagesize=letter)

    c.setFont("Helvetica-Bold", 16)
    c.drawString(1*inch, H - 1*inch, "REGULATORY COMPLIANCE AUDIT REPORT")

    c.setFont("Helvetica-Bold", 12)
    c.drawString(1*inch, H - 1.5*inch, "CONFIDENTIAL")

    y = H - 2.1*inch
    lines = [
        "Prepared by: Meridian Compliance Group LLC",
        "  Address: 555 Montgomery Street, Suite 1400, San Francisco, CA 94111",
        "",
        "Prepared for: Pacific Coast Financial Inc.",
        "  Address: 1200 Harbor Blvd, Suite 300, Oxnard, CA 93035",
        "",
        "Report Date: March 10, 2025",
        "Audit Period: January 1, 2024 through December 31, 2024",
        "Compliance Officer: Jennifer Walsh",
        "",
        "--- SCOPE ---",
        "",
        "This audit was conducted to evaluate Pacific Coast Financial Inc.'s",
        "compliance with the Dodd-Frank Wall Street Reform and Consumer Protection",
        "Act, specifically Section 1071 (Small Business Lending Data Collection),",
        "the Home Mortgage Disclosure Act (HMDA), and the Bank Secrecy Act /",
        "Anti-Money Laundering (BSA/AML) requirements.",
        "",
        "--- EXECUTIVE SUMMARY ---",
        "",
        "The audit identified three (3) findings, two of which are classified as",
        "CRITICAL and one as MAJOR. The aggregate penalty risk is estimated at",
        "up to $1,200,000. Immediate remediation is required by June 30, 2025.",
        "",
        "For reference, the previous audit (conducted March 2024) identified two (2)",
        "findings, both of which have been fully resolved.",
        "",
        "--- FINDINGS ---",
        "",
        "FINDING 1 — Missing Demographic Data [CRITICAL]",
        "  Regulation: Dodd-Frank Act, Section 1071",
        "  Description: 47 small business loan applications processed between",
        "  March and August 2024 were missing required demographic data fields",
        "  (applicant race, ethnicity, and/or gender). This represents 8.2% of",
        "  the 573 total applications during that period.",
        "  Root Cause: Software update in March 2024 inadvertently made demographic",
        "  fields optional rather than mandatory in the loan origination system.",
        "  Remediation: Restore mandatory field validation; retroactively collect",
        "  missing data where possible.",
        "",
        "FINDING 2 — HMDA Reporting Delay [MAJOR]",
        "  Regulation: Home Mortgage Disclosure Act (Regulation C)",
        "  Description: Annual HMDA data submission for calendar year 2024 was",
        "  filed 12 business days after the March 1, 2025 deadline.",
        "  Root Cause: Staff turnover in the compliance department in January 2025.",
        "  Remediation: Implement backup reporting procedures and cross-train staff.",
    ]
    y = draw_lines(c, 1*inch, y, lines, size=9.5, leading=12.5, blank_skip=8)

    # Page 2
    c.showPage()
    c.setFont("Helvetica-Bold", 11)
    c.drawString(1*inch, H - 1*inch, "Compliance Audit Report — Pacific Coast Financial (cont.)")

    y = H - 1.6*inch
    lines2 = [
        "FINDING 3 — SAR Filing Delays [CRITICAL]",
        "  Regulation: Bank Secrecy Act / Anti-Money Laundering (BSA/AML)",
        "  Description: Three (3) Suspicious Activity Reports (SARs) were filed",
        "  outside the required 30-day window following detection of suspicious",
        "  activity. The delays ranged from 8 to 22 business days beyond the",
        "  filing deadline.",
        "    SAR #1: Activity detected June 15, 2024; filed August 1, 2024 (17 days late)",
        "    SAR #2: Activity detected September 3, 2024; filed October 28, 2024 (22 days late)",
        "    SAR #3: Activity detected November 20, 2024; filed January 6, 2025 (8 days late)",
        "  Root Cause: Insufficient staffing in the BSA/AML team (2 analysts for",
        "  a portfolio requiring an estimated 3.5 FTEs).",
        "  Remediation: Hire additional BSA/AML analyst; implement automated",
        "  alert escalation for approaching deadlines.",
        "",
        "--- REMEDIATION TIMELINE ---",
        "",
        "All findings must be remediated by June 30, 2025. Pacific Coast Financial",
        "shall submit a written remediation plan to Meridian Compliance Group within",
        "30 days of this report (by April 9, 2025).",
        "",
        "--- PENALTY RISK ---",
        "",
        "Based on the severity and number of findings, the estimated aggregate",
        "penalty risk is up to $1,200,000, broken down as follows:",
        "  Finding 1 (Section 1071 violations): up to $500,000",
        "  Finding 2 (HMDA late filing): up to $200,000",
        "  Finding 3 (SAR filing delays): up to $500,000",
        "",
        "--- NEXT AUDIT ---",
        "",
        "The next scheduled compliance audit is March 2026.",
        "",
        "Prepared by:",
        "____________________________",
        "Margaret Chen, Lead Auditor",
        "Meridian Compliance Group LLC",
        "March 10, 2025",
    ]
    draw_lines(c, 1*inch, y, lines2, size=10, leading=13, blank_skip=9)

    c.save()
    print("  Created compliance_report.pdf")


# ── Main ────────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    print("Generating practice area test PDFs...")
    make_criminal_charge()
    make_divorce_decree()
    make_corporate_merger()
    make_immigration_petition()
    make_personal_injury_complaint()
    make_commercial_lease()
    make_employment_agreement()
    make_compliance_report()
    print("Done. 8 PDFs created.")

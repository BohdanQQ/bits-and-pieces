from playwright.sync_api import Playwright, Page, sync_playwright
import random, time

SEARCH_COUNT = 10

def random_search(page: Page, n: int) -> None:
    def sleepInterval(minMs, maxMs) -> None:
        time.sleep(random.randint(minMs, maxMs)/1000)
    def sleepBetweenClicks() -> None:
        sleepInterval(150, 300)
    def sleepBetweenStrokes() -> None:
        sleepInterval(80, 125)
    wordList = []
    with open('./wordlist.txt') as f:
        wordList = [ x.strip() for x in f.readlines()]

    for _ in range(n):
        word = random.choice(wordList)
        sleepBetweenClicks()
        page.get_by_role("searchbox").fill("")
        page.get_by_role("searchbox").click()
        sleepBetweenClicks()
        for c in word:
            page.get_by_role("searchbox").type(c)
            sleepBetweenStrokes()
        sleepBetweenClicks()
        page.get_by_role("searchbox").press("Enter")

def run(playwright: Playwright) -> None:
    email = "FIXME your-email-here"
    browser = playwright.chromium.launch(headless=False, executable_path='C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe')
    context = browser.new_context()
    page = context.new_page()
    page.goto("https://www.bing.com/")
    page.get_by_role("link", name="Sign in Default Profile Picture").click()
    page.get_by_placeholder("Email, phone, or Skype").click()
    page.get_by_placeholder("Email, phone, or Skype").fill(email)
    page.get_by_placeholder("Email, phone, or Skype").press("Enter")
    input("Press enter after authenticating")
    page.get_by_role("button", name="No").click()
    random_search(page, SEARCH_COUNT)
    page.close()

    context.close()
    browser.close()


with sync_playwright() as playwright:
    run(playwright)

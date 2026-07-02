#!/usr/bin/env python3

# French Chicken (Poule) ASCII Art
# Colors: Blue, White, Red (French Flag)

def print_colored_chicken():
    chicken_art = """
           \\\\
          (o o)
         (  V  )
        ^^  |  ^^
          (___)
         (_____)
        (_______)
       (_________)
    """
    
    # Print with color codes
    # Blue = \033[34m, White = \033[37m, Red = \033[31m
    print("\033[34m" + "="*50 + "\033[0m")  # Blue line
    print("\033[37m" + "            Le Coq - French Chicken" + "\033[0m")  # White text
    print("\033[31m" + "="*50 + "\033[0m")  # Red line
    print("\033[34m" + "    *" + "\033[37m" + "*" + "\033[31m" + "*" + "\033[0m")  # Blue, White, Red stars
    print(chicken_art)
    print("\033[34m" + "Blue" + "\033[37m" + " | " + "\033[31m" + "White" + "\033[37m" + " | " + "\033[31m" + "Red" + "\033[0m")  # Flag colors
    print("\033[34m" + "Liberté" + "\033[37m" + " | " + "\033[31m" + "Égalité" + "\033[37m" + " | " + "\033[31m" + "Fraternité" + "\033[0m")  # French motto

if __name__ == "__main__":
    print_colored_chicken()
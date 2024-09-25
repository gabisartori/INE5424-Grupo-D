

def main():
    file = open("tests/log.txt", "r")
    total_sends = 0
    total_receivs = 0
    expected_sends = 0
    expected_receivs = 0
    for line in file.readlines():
        words = line.split()
        sends = [int(i) for i in words[4].split("/")]
        receivs = [int(i) for i in words[7].split("/")]
        total_sends += sends[0]
        total_receivs += receivs[0]
        expected_sends += sends[1]
        expected_receivs += receivs[1]
    print(f"Total de Pacotes Enviados : {total_sends}/{expected_sends}")
    print(f"Total de Pacotes Recebidos: {total_receivs}/{expected_receivs}")
        

if __name__ == "__main__":
    main()

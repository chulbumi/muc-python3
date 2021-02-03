# -*- coding: utf-8 -*-

from lib.func import getInt, stripANSI, getNextWords
from lib.hangul import is_han

class AutoScript:
    def start(self, script, player):
        self.tick = 0
        self.lineNum = 0
        self.player = player
        self.script = script
        self.lastNum = len(script)
        self.run()
        self.name = ''
        
    def run(self):
        print(self.player['이름'] + ' %d/%d' %(self.lineNum, self.lastNum))
        if self.player is None:
            return
        printLine = False
        loopcount = 0
        while(True):
            if loopcount > 50:
                print('Exceed loop count in autoscript.py : lineNum = %d' % self.lineNum)
                self.player.sendLine('* 취소합니다.')
                self.player.stopAutoScript()
                return
            
            if self.lineNum >= self.lastNum:
                self.player.stopAutoScript()
                return
            line = self.script[self.lineNum].strip()
            if line == '':
                self.player.sendLine('')
            elif line[0] == '#':
                self.lineNum += 1
                continue
            elif line[0] == '$':
                l = line.strip()
                nw = getNextWords(l)
                if l == '$출력시작':
                    printLine = True
                elif l == '$출력끝':
                    printLine = False
                    self.lineNum += 1
                    continue
                elif l.startswith('$종료'):
                    self.player.stopAutoScript()
                    return
                elif l.startswith('$틱'):
                    tick = getInt(nw)
                    if tick != 0:
                        self.tick = tick * 0.1 * 1.5
                    self.lineNum += 1
                    continue
                elif l.startswith('$키입력'):
                    key = l.find(':')
                    if key == -1:
                        self.player.input_to(self.player.pressEnter1)
                    else:
                        self.player.input_to(self.player.getKeyInput, l[key + 1:])
                    self.lineNum += 1
                    return
                elif l.startswith('$한글확인'):
                    if not is_han(stripANSI(self.player.temp_input)):
                        self.lineNum -= getInt(nw)
                        self.player.sendLine('한글만 입력 가능합니다. 다시 입력하세요.')
                        continue
                elif l.startswith('$단어입력'):
                    words = nw
                    limit = 10
                    keywords = []
                    if len(words) > 0:
                        limit = getInt(words[0])
                        if len(words) == 2:
                            keywords = words[1].split(',')
                    self.lineNum += 1
                    self.player.input_to(self.player.getWord, limit, keywords)
                    return
                elif l.startswith('$한줄입력'):
                    self.lineNum += 1
                    self.player.input_to(self.player.getLine, nw)
                    return
                elif l.startswith('$라인입력'):
                    self.lineNum += 1
                    self.player.temp_input = []
                    self.player.sendLine('입력을 마치시려면 \'.\' 를 입력하세요.')
                    self.player.input_to(self.player.getLines, nw)
                    return
                elif l.startswith('$입력확인'):
                    self.lineNum += 1
                    from builtins import weapon_type
                    if weapon_type(self.player.temp_input) == list:
                        self.player.write('입력하신 내용이 맞습니까? (네/취소) : ')
                    else:
                        self.player.write('입력하신 내용이 \'' + self.player.temp_input + '\' 맞습니까? (네/취소) : ')
                    self.player.input_to(self.player.checkInput)
                    return
                elif l.startswith('$중복확인'):
                    find = False
                    for index in Item.Items:
                        item = Item.Items[index]
                        if item['이름'] == stripANSI(self.player.temp_input):
                            self.lineNum -= getInt(nw)
                            self.player.sendLine('중복된 이름이 있습니다. 다시 입력하세요.')
                            find = True
                            break
                    if find == True:
                        continue
                elif l.startswith('$무기생성'):
                    self.player.temp_item = getItem('올숙무기').deepclone()
                    self.player.temp_item.index = self.player['이름'] + '_올숙무기'
                    self.player.temp_item.path = 'data/item/' + self.player.temp_item.index + '.itm'
                    weapon_type = getInt(self.player.temp_input)
                    self.player.temp_item['무기종류'] = weapon_type
                    if weapon_type == 1:
                        msg = '검'
                    elif weapon_type == 2:
                        msg = '도'
                    elif weapon_type == 3:
                        msg = '창'
                    elif weapon_type == 4:
                        msg = '기타'
                    elif weapon_type == 5:
                        msg = '주먹1'
                    self.player.temp_item['전투스크립'] = msg
                    self.player.temp_item['사용자'] = self.player['이름']
                elif l.startswith('$무기속성'):
                    if l.startswith('이름'):
                        self.player.temp_item[nw] = self.player.temp_input
                        self.player.temp_item['반응이름'] = [ stripANSI(self.player.temp_input) ]
                    elif l.startswith('설명2'):
                        self.player.temp_item['설명2'] = ''
                        for li in self.player.temp_input:
                            self.player.temp_item['설명2'] += li + '\r\n'
                    elif l.startswith('무공이름'):
                        mugong = self.player.temp_input
                        jung = ['철사장', '격공장', '낙화유수', '격공진력', '난화불혈수', '삼절연환', '복호장', '벽공장', '한빙장', '동해신장', '화선유불장',
                                '청산장', '백보신권', '탄지신공', '탈명구식', '대력금강장', '반야대수인', '천세혈격', '매화난무', '혼천공', '뇌격신룡참', '무극대력',
                                '무형일식', '난화만번', '파천검', '일지선공', '벽력진식', '추창망월', '무영신공', '교탈조화', '관음장', '삼화취정장', '천수대라인',
                                '뇌령일식', '현천무극강', '달마삼검', '표검향우', '대비단혼강', '뇌음자흑강', '만불조종', '무극검']

                        sa = ['신천호패', '동추수', '광표권', '합마공', '혈천공', '묵혈강', '빙혈신장', '구상검', '파혈공', '풍마장', '혈산설화', '화우폭진',
                              '녹수마장', '소수겁', '혈화비천', '현마장', '혈화장', '혈천섬광', '밀종대수인', '대수인', '혈월비적', '뇌격신룡참', '만천화우',
                              '비혈장', '불천격뇌', '묵령대혼', '무진폭마', '격천살', '금황마라수', '수라폭', '혈지공', '잔살혈영공', '멸천혈폭', '천마폭',
                              '천마무격신장', '수라멸', '천마검', '혈세천하']
                        if mugong in jung:
                            weapon_type = '정파'
                        elif mugong in sa:
                            weapon_type = '사파'
                        else:
                            weapon_type = '정사'

                        self.player.temp_item['무공이름'] = '%s %s 1000000 10000000 10' % (mugong, weapon_type)
                    else:
                        self.player.temp_item[nw] = self.player.temp_input
                elif l.startswith('$숙련도선택'):
                    if self.player.temp_input == '1':
                        self.player.temp_move = 1
                    else:
                        self.player.temp_move = 0
                elif l.startswith('$숙련도이전'):
                    if self.player.temp_move == 1:
                        main = self.player.temp_item['무기종류']
                        exp = 0
                        for i in range(1, 6):
                            if i == main:
                                continue
                            s = self.player['%d 숙련도' % i]
                            exp += self.player['%d 숙련도경험치' % i]
                            for i in range(0, s + 1):
                                exp += (i + 5) * 7
                        s = self.player['%d 숙련도' % main]
                        a = 0
                        while(True):
                            e = (s + a + 1 + 5) * 7 
                            if exp - e < 0:
                                break
                            exp -= e
                            a += 1
                        self.player['%d 숙련도' % main] = s + a
                        for i in range(1, 6):
                            if i == main:
                                continue
                            self.player['%d 숙련도' % i] = 0
                            self.player['%d 숙련도경험치' % i] = 0
                        self.player.sendLine('☞ 숙련도 경험치가 옮겨졌습니다.')
                elif l.startswith('$무기지급'):
                    self.player['올숙완료'] = 1
                    self.player.temp_item.save()
                    del self.player.temp_item
                    self.player.temp_item = None
                    item = getItem(self.player['이름'] + '_올숙무기').deepclone()
                    self.player.objs.append(item)
                    self.player.save()
                elif l.startswith('$아이템삭제'):
                    index, cnt = getStrCnt(line)
                    self.player.delItem(index, cnt)
                elif l.startswith('$아이템확인'):
                    if not self.player.checkItemName(self.player.temp_input, 1):
                        self.player.sendLine('☞ 그런 아이템이 소지품에 없어요.')
                        self.player.sendLine('* 무기강화를 종료합니다.')
                        self.player.stopAutoScript()
                        return
                    self.player.temp_item = self.player.getItemName(self.player.temp_input)
                    """
                        self.player.sendLine('☞ 무기만 가능합니다.')
                        self.player.sendLine('* 무기강화를 종료합니다.')
                        self.player.stopAutoScript()
                        return
                    """
                elif l.startswith('$옵션확인'):
                    if self.player['최고내공'] < 10:
                        self.player.sendLine('☞ 내공이 부족해요.')
                        self.player.sendLine('* 무기강화를 종료합니다.')
                        self.player.stopAutoScript()
                        return

                    op = self.player.temp_input
                    item = self.player.temp_item
                    option = item.getOption()
                    if op == '방어력':
                        self.player.sendLine('☞ 해당 특성치는 안되요.')
                        self.player.sendLine('* 무기강화를 종료합니다.')
                        self.player.stopAutoScript()
                        return

                    if op not in option:
                        self.player.sendLine('☞ 그런 특성치는 없어요.')
                        self.player.sendLine('* 무기강화를 종료합니다.')
                        self.player.stopAutoScript()
                        return
                    val = option[op]
                    myItem = self.player.getItemIndex(self.player['이름'] + '_올숙무기')
                    if myItem == None or myItem.inUse == True:
                        self.player.sendLine('☞ 무기를 벗고 하세요.')
                        self.player.sendLine('* 무기강화를 종료합니다.')
                        self.player.stopAutoScript()
                        return
                    myOp = myItem.getOption()
                    if myOp == None:
                        myOp = {}
                    if op not in myOp:
                        myVal = 0
                    else:
                        myVal = myOp[op]
                    if myVal >= val:
                        self.player.sendLine('☞ 현재 특성치 값보다 높아야 합니다.')
                        self.player.sendLine('* 무기강화를 종료합니다.')
                        self.player.stopAutoScript()
                        return

                    d = val - myVal
                    if d > 10:
                        d = 10     
                    myOp[op] = myVal + d
                    myItem.setOption(myOp)
                    if myItem['공격력'] < 9999:
                        myItem['공격력'] = myItem['공격력'] + 10
                        if myItem['공격력'] > 9999:
                            myItem['공격력'] = 9999 
                        myItem['기량'] = myItem['공격력']
                    self.player['최고내공'] = self.player['최고내공'] - 10 
                    self.player.delItem(item.index, 1)
                    if myItem['공격력'] > 2000:
                        self.player.delItem('강철판', 5)
                    myItem.save()
                    self.player.save()

                elif l.startswith('$옵션출력'):
                    item = self.player.temp_item
                    if item == None:
                        self.player.sendLine('* 무기강화를 종료합니다.')
                        self.player.stopAutoScript()
                        return
                    option = item.getOption()
                    if option is None:
                        self.player.sendLine('☞ 해당 아이템은 특성치가 없어요.')
                        self.player.sendLine('* 무기강화를 종료합니다.')
                        self.player.stopAutoScript()
                        return
                    self.player.sendLine(item.getOptionStr())
                elif l.startswith('$무기강화'):
                    pass
                
            else:
                #msg = line.replace('[공]', self.player['이름'])
                #self.player.write(postPosition1(msg))
                self.player.sendLine(line)
            self.lineNum += 1
            if printLine == True:
                continue
            if self.tick == 0:
                continue
            reactor.callLater(self.tick, self.run)
            break

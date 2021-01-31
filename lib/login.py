# -*- coding: utf-8 -*-

from include.path import *
from include.define import *
from lib.object import *
from lib.hangul import *
from objs.cmd import Command
from lib.comm import broadcast
from lib.cmd import parse_command

def userlist(ob):
    list = '총 (' + str(len(ob.clients)) + ')\r\n'        
    #list = ''
    for c in ob.channel.clients:
        if len(c.get('이름')) != 0:
            list += ', ' + c.get('이름')
        else: 
            list += ', <접속중>'
                                                                   
    ob.sendLine(list);


def get_name(self, name, *args):
    if len(name) == 0:
        self.write('\r\n이름 : ')
        return
    if is_han(name) == False:
        self.write('\r\n한글 입력만 가능합니다.\r\이름 : ')
        return
    if name == '손님':
        self.newidx = 0
        newbie_msg(self)
        self.input_to(DoNothing)
        return
    
    res = self.load(USER_PATH + name)
    if res == False:
        self.write('\r\n그런 사용자는 없습니다.\r\n이름 : ')
        return
    #self.set('이름', name)
    self.write('\r\n암호 : ')
    self.loginRetry = 0
    self.input_to(get_pass)


def get_pass(self, line, *args):
    self.loginRetry += 1
    if len(line) == 0 or self.get('암호') != line:
        if self.loginRetry >= 3:
            self.write('\r\n')
            self.channel.transport.loseConnection()
            return
        self.write('\r\n잘못된 암호 입니다.\r\n암호 : ')
        return    
    #self.sendLine('\r\n또 오셨구만요^^ 반가워요')
    del self.loginRetry
    
    self.write('[2;28r[2J')

    self.state = ACTIVE
    self.channel.clients.append(self)
    self.channel.echoON()
    broadcast(self.get('이름') + '님이 들어오셨습니다.', self)
    
    from lib.io import cat
    cat(self, 'data/text/notice.txt')
    self.sendLine('[Enter] 키를 누르세요.')
    self.input_to(showNotice)
    self.start_heart_beat()


def showNotice(self, line, *args):
    room = get_room('시작/시작')
    if room != None:
        room.append(self)

    self.INTERACTIVE = 1
    self.input_to(parse_command)


def newbie_msg(self):
    from twisted.internet import reactor
    
    self.newidx += 1
    
    if self.newidx == 1:
        self.write('[2J') # CLEAR SCREEN
    elif self.newidx == 2:
        self.sendLine('\r\n옷깃을 가볍게 적시는 가랑비가 촉촉히 내리는 새벽 .......')
    elif self.newidx == 3:
        self.sendLine('\r\n어둠을 깨트리는 처절한 비명 소리를 뒤로 하고 생사를 건 탈출을 하는 이들이')
        self.sendLine('있었다.')
    elif self.newidx == 4:
        self.sendLine('\r\n두 명의 사내와 갓 세살을 넘었을 만한 아이....')
    elif self.newidx == 5:
        self.sendLine('\r\n한 남자는 중년의 건장한 모습이나 온통 피로 물들어 있었고 다른 한 남자는')
        self.sendLine('심한 부상을 입은 듯 복부를 왼손으로 감싸고 오른 손엔 검을 쥐고 있으나')
        self.sendLine('어깨에서 부터 흘러내린 피가 검신을 타고 끊임 없이 흘러 내리고 있었다.')
    elif self.newidx == 6:
        self.sendLine('\r\n일주가 말합니다. "소룡!, 나는 더 이상 갈수가 없을 것 같네...".')
        self.sendLine('                 "어서 소주인을 모시고  이 곳을 빠져나가게 ....."')
    elif self.newidx == 7:
        self.sendLine('\r\n                 "어서 소주인을 모시고  이 곳을 빠져나가게 ....."')
    elif self.newidx == 8:
        self.sendLine('\r\n쿵쿵따... 쿵쿵따... 주저리...주저리...')
    elif self.newidx == 9:
        self.sendLine('\r\n쿵쿵따... 쿵쿵따... 주저리...주저리...')
    elif self.newidx == 10:
        self.sendLine('\r\n쿵쿵따... 쿵쿵따... 주저리...주저리...')
    elif self.newidx == 11:
        self.sendLine('\r\n쿵쿵따... 쿵쿵따... 주저리...주저리...')
    elif self.newidx == 12:
        self.sendLine('\r\n쿵쿵따... 쿵쿵따... 주저리...주저리...')
    elif self.newidx == 13:
        self.sendLine('\r\n[Enter] 키를 누르세요.\r\n')
        self.input_to(NextPage)
        return
    elif self.newidx == 14:
        self.sendLine('\r\n그러던 어느날...')
    elif self.newidx == 15:
        self.sendLine('\r\n쿵쿵따... 쿵쿵따... 주저리...주저리...')
    elif self.newidx == 16:
        self.sendLine('\r\n쿵쿵따... 쿵쿵따... 주저리...주저리...')
    elif self.newidx == 17:
        self.sendLine('\r\n사용할 이름을 입력하세요.\r\n이름 : ')
        self.input_to(getNewname)
        return
    
    reactor.callLater(1, newbie_msg, self)


def DoNothing(self, line, *args):
    return


def NextPage(self, line, *args):
    from twisted.internet import reactor
    self.write('[2J') # CLEAR SCREEN
    self.input_to(DoNothing)
    reactor.callLater(3, newbie_msg, self)
    return
    

def getNewname(self, name, *args):
    if len(name) == 0:
        self.write('\r\n한글자 이상 입력하세요.\r\n이름 : ')
        return
    if is_han(name) == False:
        self.write('\r\n한글 입력만 가능합니다.\r\n이름 : ')
        return
    if name == '손님':
        self.write('\r\n사용할 수 없는 이름입니다.\r\n이름 : ')
        return
    import os
    if os.path.exists(USER_PATH + name) == True:
        self.write('\r\n이미 사용중인 이름입니다.\r\n이름 : ')
        return
    self.set('이름', name)
    self.write('\r\n사용하실 암호를 입력하세요.\r\n암호 : ')
    self.input_to(getNewpass)

def getNewpass(self, line, *args):
    if len(line) < 3:
        self.write('\r\n3자 이상 입력하세요.\r\n암호 : ')
        return
    self.set('암호', line)
    self.write('\r\n한번 더 입력하세요.\r\n암호 : ')
    self.input_to(getNewpass2)


def getNewpass2(self, line, *args):
    if line != self.get('암호'):
        self.write('\r\n이전 입력과 다릅니다.\r\n사용하실 암호를 입력하세요.\r\n암호 : ')
        self.input_to(getNewpass)
        return
    self.init_body()
    self.save(USER_PATH + self.get('이름'))
    self.write('[2;28r[2J')
    self.state = ACTIVE
    self.channel.clients.append(self)
    self.channel.echoON()
    broadcast(self.get('이름') + '님이 들어오셨습니다.', self)
    
    from lib.io import cat
    cat(self, 'data/text/notice.txt')
    self.sendLine('[Enter] 키를 누르세요.')
    self.input_to(showNotice)
    self.start_heart_beat()

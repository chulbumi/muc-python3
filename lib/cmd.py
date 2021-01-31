# -*- coding: utf-8 -*-

import sys, traceback
from lib.object import *
from lib.comm import broadcast

def parse_command(self, line, *args):
    if len(line) == 0:
        return
        
    from objs.player import Player
    if line[-1] in (' ', '.', '!', '?', ','):
        Player.cmdList['말'].cmd(self, line)
        return

    cmd = line.split(' ')[-1]
    param = line.rstrip(cmd)
    param = param.strip()

    s = self.get('줄임말')
    if s != None and cmd in s:
        wlist = s[cmd].split(';')
        cmd = wlist[0]
        msg = ''
        for w in wlist[1:]:
            #if w in s:
            #    self.sendLine('중첩된 줄임말은 사용할 수 없습니다.')
            #    return
            msg += w + '\r\n'
        self.channel.buffer = msg + self.channel.buffer

    from objs.alias import alias
    if cmd in alias:
        cmd = alias[cmd]
                
    if type(self.env.get('출구')) is dict and cmd in self.env.get('출구'):
        if self.is_attack():
            self.sendLine('전투 중에는 이동 할 수 없습니다.')
            return

        exit = self.env.get('출구')[cmd]
        if type(exit) is list:
            import random
            r = random.randint(0, len(exit)-1)
            exit = exit[r]
        room = get_room(exit)
      
        if room == None:
            self.sendLine('* 이동 실패!!!')
            return
        room.append(self, cmd)
        return
    elif cmd in ('끝', '종료'):
        if self.is_attack():
            self.sendLine('전투 중에는 종료 할 수 없습니다.')
            return
        self.sendLine('다음에 또 만나요~!!!')
        broadcast(self.get('이름') + '님이 나가셨습니다.', self)
        self.channel.transport.loseConnection()
        return
    elif cmd in Player.emotes:
        try:
            Player.emotes[cmd].cmd(self, param)
        except :
            traceback.print_exc(file=sys.stderr)
            print 'Error in %s' % cmd
        return
    elif cmd in Player.cmdList:
        try:
            Player.cmdList[cmd].cmd(self, param)
        except :
            traceback.print_exc(file=sys.stderr)
            print 'Error in %s' % cmd
        
        # 사용자의 명령을 처리후 몹 엑션 후크
        for obj in self.env.objs:
            if is_mob(obj) and hasattr(obj, 'mob_hook'):
                obj.mob_hook(line)
        return


    # 룸 오브젝트의 액션을 처리
    if hasattr(self.env, 'actions'):
        if cmd in self.env.actions:
            self.env.actions[cmd](self, param)
            return

    # 룸에 있는 오브젝트들의 액션을 처리
    for obj in self.env.objs:
        if hasattr(obj, 'actions'):
            if cmd in obj.actions:
                obj.actions[cmd](self, param)
                return
           
    self.sendLine('\r\n☞ 무슨 말인지 모르겠어요. *^_^*')
    
def pressEnter(self, line, *args):
    self.INTERACTIVE = 1
    self.input_to(parse_command)

# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    room_num = [
        12,14,16,18,20,
        34,36,38,40,42,
        56,58,60,62,64,
        78,80,82,84,86,
        100,102,104,106,108,
    ]


    def cmd(self, ob, line):
　　　　self.res = [ 
　　　　　　'　','　','　','　','　','　','　','　','　','　','　',
　　　　　　'　','　','　','　','　','　','　','　','　','　','　',
　　　　　　'　','　','　','　','　','　','　','　','　','　','　',
　　　　　　'　','　','　','　','　','　','　','　','　','　','　',
　　　　　　'　','　','　','　','　','　','　','　','　','　','　',
　　　　　　'　','　','　','　','　','　','　','　','　','　','　',
　　　　　　'　','　','　','　','　','　','　','　','　','　','　',
　　　　　　'　','　','　','　','　','　','　','　','　','　','　',
　　　　　　'　','　','　','　','　','　','　','　','　','　','　',
　　　　　　'　','　','　','　','　','　','　','　','　','　','　',
　　　　　　'　','　','　','　','　','　','　','　','　','　','　',
　　　　　　'　','　','　','　','　','　','　','　','　','　','　',
　　　　]

        if ob.env == None:
            ob.sendLine('\r\n* 아무것도 보이지 않습니다.\r\n')
            return
        c = 0
        for exitName in ob.env.exitList:
            if exitName[-1] == '$':
                continue
            c += 1
        if c == 0:
            ob.sendLine('☞ 아무것도 보이지 않습니다.')
            return
        self.exit_mark(ob.env, 60)
        ob.write(self.map())

    def map(self):
        maptext=''
        j = 0
        for i in range(len(self.res)):
            j += 1
            maptext += self.res[i];
            if j == 11: 
                maptext += '\r\n';
                j = 0;
        return maptext

    def exit_mark(self, room, roomnum):
        if room == None:
            return
        if room.Exits == None:
            return
        if roomnum not in self.room_num:
            return
        if roomnum >= len(self.res):
            return
        if roomnum < 0:
            return
        
        if self.res[roomnum] == '  ':
            if roomnum == 60:
                self.res[roomnum] = '[1;33m○[37;0m'
            else:
                self.res[roomnum] = '○'
        else:
            return
        exits = room.Exits
        
        for exitName in exits:
            if exitName == '동': 
                if roomnum + 1 >= 132:
                    continue
                if self.res[roomnum+1] == '　':
                    self.res[roomnum+1] = '→'
                else:
                    self.res[roomnum+1] = '─'
                
                self.exit_mark(room.getExit1(exitName), roomnum+2)
            elif exitName == '서': 
                if roomnum - 1 < 0:
                    continue
                if self.res[roomnum-1] == '　':
                    self.res[roomnum-1] = '←'
                else:
                    self.res[roomnum-1] = '─'
                
                self.exit_mark(room.getExit1(exitName), roomnum-2)
            elif exitName == '남': 
                if roomnum + 11 >= 132:
                    continue
                if self.res[roomnum+11] == '　':
                    self.res[roomnum+11] = '↓'
                else:
                    self.res[roomnum+11] = '│'
                self.exit_mark(room.getExit1(exitName), roomnum+22)
            elif exitName == '북': 
                if roomnum - 11 < 0:
                    continue
                if self.res[roomnum-11] == '　':
                    self.res[roomnum-11] = '↑'
                else:
                    self.res[roomnum-11] = '│'
                self.exit_mark(room.getExit1(exitName), roomnum-22)
            elif exitName == '북동': 
                if roomnum - 10 < 0:
                    continue
                if self.res[roomnum-10] == '　':
                    self.res[roomnum-10] = '↗'
                else:
                    self.res[roomnum-10] = '／'
                self.exit_mark(room.getExit1(exitName), roomnum-20)
            elif exitName == '북서': 
                if roomnum - 12 < 0:
                    continue
                if self.res[roomnum-12] == '　':
                    self.res[roomnum-12] = '↖'
                else:
                    self.res[roomnum-12] = '＼'
                self.exit_mark(room.getExit1(exitName), roomnum-24)
            elif exitName == '남동':
                if roomnum + 12 >= 132:
                    continue
                if self.res[roomnum+12] == '　':
                    self.res[roomnum+12] = '↘'
                else:
                    self.res[roomnum+12] = '＼'
                self.exit_mark(room.getExit1(exitName), roomnum+24)
            elif exitName == '남서': 
                if roomnum + 10 >= 132:
                    continue
                if self.res[roomnum+10] == '　':
                    self.res[roomnum+10] = '↙'
                else:
                    self.res[roomnum+10] = '／'
                self.exit_mark(room.getExit1(exitName), roomnum+20)
            elif exitName == '위': 
                if roomnum == 60:
                    if self.res[roomnum] == '[1;33m○[37;0m':
                        self.res[roomnum] = '[1;33m∧[37;0m'
                    else:
                        self.res[roomnum] = '[1;33m↕[37;0m'
                else:
                    if self.res[roomnum] == '○':
                        self.res[roomnum] = '∧'
                    else:
                        self.res[roomnum] = '↕'
            elif exitName == '아래' or exitName == '밑': 
                if roomnum == 60:
                    if self.res[roomnum] == '[1;33m○[37;0m':
                        self.res[roomnum] = '[1;33m∨[37;0m'
                    else:                                     
                        self.res[roomnum] = '[1;33m↕[37;0m'
                else:
                    if self.res[roomnum] == '○':
                        self.res[roomnum] = '∨'
                    else:
                        self.res[roomnum] = '↕'


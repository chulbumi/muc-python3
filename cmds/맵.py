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


    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        words = line.split()
        if len(words) < 1:
            ob.sendLine('☞ 사용법: [제외할방향] 맵')
            return
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
        if words[0] not in ob.env.exitList:
            ob.sendLine('☞ 그 방향으로는 갈수가 없어요!.')
            return

        ob.walkhis = []
        ob.mapQ = {}
        l = copy.copy(ob.env.exitList)
        l.remove(words[0])
        a = []
        for ex in l:
            if ex in ['동', '서', '남', '북', '북서', '북동', '남서', '남동']:
                a.append(ex)
        l = a
        ob.mapQ[ob.env.index] = []
        if len(l) == 0:
            ob.sendLine('☞ 그 방향으로는 갈수가 없어요!.')
            del ob.mapQ
            del ob.walkhis
            return
        self.explorer(ob, ob.env.getExit(l[0]), l[0])
        ob.write(self.map(ob.walkhis))
        #print ob.walkhis

        del ob.mapQ
        del ob.walkhis

    def count_explorer(self, ob, env, direction):
        if env == None:
            return 0

        reverse = self.reverseDir(direction)
        if reverse == '':
            return 0

        if env.index in ob.tempQ:
            return 0
        else:
            l = copy.copy(env.exitList)
            ob.tempQ[env.index] = l

        if reverse in l:
            l.remove(reverse)

        if len(l) == 0:
            return 1

        n = 0
        for exit in l:
            if exit not in ['동', '서', '남', '북', '북서', '북동', '남서', '남동']:
                continue
            r = env.getExit(exit)
            if r == None:
                continue
            if env.zone != r.zone:
                continue
            n += self.count_explorer(ob, r, exit)

        ob.tempQ[env.index] = []
        return n

    def explorer(self, ob, env, direction):
        if env == None:
            print('env is None')
            return
        #print env.index, direction

        reverse = self.reverseDir(direction)
        if reverse == '':
            return

        if env.index in ob.mapQ:
            l = ob.mapQ[env.index]
            if reverse in l:
                l.remove(reverse)
            return
            #l = ob.mapQ[env.index]
        else:
            l = copy.copy(env.exitList)
            ob.mapQ[env.index] = l

        if reverse in l:
            l.remove(reverse)
        ob.walkhis.append(direction)

        dirs = {}
        for exit in l:
            ob.tempQ = {}
            if exit not in ['동', '서', '남', '북', '북서', '북동', '남서', '남동']:
                continue
            r = env.getExit(exit)
            if r == None:
                print(env.index, exit)
                continue
            if env.zone != r.zone:
                continue
            n = self.count_explorer(ob, r, exit)
            dirs[exit] = n
        import operator
        sorted_x = sorted(iter(dirs.items()), key=operator.itemgetter(1))
        sorted_l = []
        for item in sorted_x:
            sorted_l.append(item[0]) 

        l = sorted_l
        while(True):
            if len(l) == 0:
                break
            exit = l.pop()
            if exit not in ['동', '서', '남', '북', '북서', '북동', '남서', '남동']:
                continue
            r = env.getExit(exit)
            if r == None:
                print(env.index, exit)
                continue
            if env.zone != r.zone:
                continue
            self.explorer(ob, r, exit)

        ob.mapQ[env.index] = []

        if self.checkEmpty(ob.mapQ) == False:
            ob.walkhis.append( reverse )
        return

    def checkEmpty(self, mapQ):
        for m in mapQ:
            l = mapQ[m]
            for exit in l:
                #print m, exit
                return False
        return True

    def reverseDir(self, direction):
        if direction == '동':
            return '서'
        if direction == '서':
            return '동'
        if direction == '남':
            return '북'
        if direction == '북':
            return '남'
        if direction == '북서':
            return '남동'
        if direction == '남동':
            return '북서'
        if direction == '북동':
            return '남서'
        if direction == '남서':
            return '북동'
        if direction == '위':
            return '아래'
        if direction == '아래':
            return '위'
        return ''

    def map(self, l):
        maptext=''
        for i in l:
            maptext += i + ';';
        return maptext


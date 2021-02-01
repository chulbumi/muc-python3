import _pickle as cPickle
from objs.object import Object
import json


class Guild(Object):
    
    attr = {}
    path = 'data/config/guild.dat'
    def __init__(self):
        self.load()
        
    def load(self):
        try:
            f = open(self.path, "rb")
            self.attr = cPickle.load(f, encoding="euc-kr")
        except IOError:
            print('%s IOError' % self.path)
            return
        except EOFError:
            print('%s EOFError' % self.path)
            return
        except:
            print('Error %s' % self.path)
            return
        f.close()
    
    def save(self):
        try:
            f = open(self.path, 'wb')
            cPickle.dump(self.attr, f)
        except IOError:
            print('%s IOError' % self.path)
            return
        except EOFError:
            print('%s EOFError' % self.path)
            return
        except:
            print('Error %s' % self.path)
            return

GUILD = Guild()

        
